# Crash reporting and metrics

[Documentation index](README.md)

QuickGUI implements crash reporting and process/system metrics in the Rust core
(`quickgui-system`) and retains frame metrics in each window runtime. The Go and
TypeScript bindings expose those readings explicitly. Nothing here runs in the
background unless you explicitly start the opt-in watchdog.

## Crash reporting

`CrashReporter::install` sets a process-wide panic hook and, unless disabled, a native fatal-fault
handler. Both write one bounded JSON report per crash into a retained directory.

```rust
use quickgui::{AppInfo, BacktracePolicy, CrashReporter, CrashReporterOptions};

let app = AppInfo::new("Demo", env!("CARGO_PKG_VERSION"), "com.example.demo")?;
let reporter = CrashReporter::install(
    CrashReporterOptions::new(app)
        .max_reports(16)?
        .max_report_bytes(64 * 1024)?
        .extra_parameter("channel", "beta")?
        .upload_endpoint("https://crash.example.com/report")?
        .backtrace(BacktracePolicy::Environment),
)?;

if let Some(report) = reporter.last_crash_report()? {
    eprintln!("previous run crashed: {} ({})", report.message, report.timestamp);
    reporter.delete_report(&report.id)?;
}
```

The default directory is `<AppPaths::log_dir>/crashes`, falling back to
`<AppPaths::temp_dir>/crashes` on a platform with no log directory. Installing twice is an error;
`CrashReporter::current()` returns the installed handle. `CrashReporter::detached(options)` builds
a reporter that owns a directory but installs no process-wide hooks — used by tests and by tools
that only read or upload reports.

### Report contents

| Field | Panic | Native fault | Hang |
| --- | --- | --- | --- |
| `schemaVersion`, `id`, `timestamp` (RFC 3339 UTC) | yes | yes | yes |
| `appName`, `appVersion`, `appIdentifier` | yes | yes | yes |
| `operatingSystem`, `operatingSystemVersion`, `architecture`, `processId` | yes | yes | yes |
| `thread` | thread name | `pthread_self` as hex | thread name |
| `message` | panic payload | fixed text | threshold text |
| `location` (file/line/column) | yes | no | no |
| `backtrace` | policy-dependent | no | no |
| `signal`, `signalName`, `faultAddress` | no | yes | no |
| `parameters` | yes | snapshot taken at install time | yes |

### Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `CRASH_REPORT_SCHEMA_VERSION` | `1` | Stamped into every report |
| `MAX_CRASH_REPORTS` | `128` | Ceiling for `max_reports` |
| `DEFAULT_MAX_CRASH_REPORTS` | `16` | Retention when unset |
| `MAX_CRASH_REPORT_BYTES` | `1 MiB` | Ceiling for `max_report_bytes` |
| `DEFAULT_MAX_CRASH_REPORT_BYTES` | `64 KiB` | Per-report limit when unset |
| `MAX_CRASH_EXTRA_PARAMETERS` | `32` | Retained key/value pairs |
| `MAX_CRASH_PARAMETER_KEY_BYTES` | `128` | One parameter key |
| `MAX_CRASH_PARAMETER_VALUE_BYTES` | `4 KiB` | One parameter value |
| `MAX_CRASH_MESSAGE_BYTES` | `8 KiB` | Panic/hang message, truncated |
| `MAX_CRASH_BACKTRACE_BYTES` | `32 KiB` | Backtrace, truncated |
| `MAX_CRASH_UPLOAD_REPORTS` | `32` | Reports sent per `upload_pending` call |

A report that still exceeds `max_report_bytes` after serialization is retried without its
backtrace and parameters and with its message trimmed to 1 KiB; if it still does not fit it is
rejected rather than truncated into invalid JSON. Retention deletes the oldest reports (report
identifiers begin with a zero-padded millisecond timestamp, so filename order is chronological).

### The native fault path

Reports for `SIGSEGV`, `SIGBUS`, `SIGILL`, `SIGFPE`, and `SIGABRT` are written from an
async-signal-safe handler. Everything the handler needs is prepared while the process is healthy:

- a scratch file (`<id>.crash.partial`) is created and its descriptor kept open;
- the whole invariant part of the JSON — schema, identity, OS, architecture, process ID, message,
  and the extra parameters as they stood at install time — is rendered once into a byte buffer.

The handler only writes that buffer, decimal and hexadecimal digits it formats on the stack, and
an RFC 3339 timestamp derived from `time(NULL)` with integer arithmetic; it uses `write` and
`fsync` and nothing else. It never allocates, never locks, and never calls into `std`'s formatting
machinery. `SA_SIGINFO | SA_ONSTACK | SA_RESETHAND` is installed with `sigaction`, so the handler
runs once and the following `raise` terminates the process with the original fault.

The next `CrashReporter::install`/`detached` promotes any non-empty `*.crash.partial` to a real
report and deletes empty ones whose process is no longer alive (`kill(pid, 0)`).

Windows uses `SetUnhandledExceptionFilter` and writes the same field layout with the exception
code and address in place of the signal number and fault address. The Windows path is minimal by
design and, unlike the Unix path, has not been exercised on a Windows host by this change.

| Platform | Panic hook | Native fault capture | Extra parameters in fault reports |
| --- | --- | --- | --- |
| macOS | yes | `sigaction`, async-signal-safe | snapshot from install time |
| Linux | yes | `sigaction`, async-signal-safe | snapshot from install time |
| Windows | yes | `SetUnhandledExceptionFilter` (untested on a Windows host) | snapshot from install time |
| Other | yes | none | — |

### Upload

`upload_pending(endpoint)` POSTs each retained report as `application/json` over HTTPS and deletes
only the ones the endpoint accepts with a 2xx status. Plain HTTP is rejected before any request.
It returns `UploadSummary { attempted, uploaded, failed }` and never sends more than
`MAX_CRASH_UPLOAD_REPORTS` reports per call. The `crash-reporter` feature owns the optional
`serde`/`serde_json`/`ureq` dependencies; building without it removes the module entirely.

### The watchdog is opt-in

`Watchdog` is **disabled by default**. Starting it costs exactly one sleeping thread that wakes
every `interval` (default 5 s, range 10 ms – 300 s) and compares the last heartbeat against
`hang_threshold` (default 15 s). At the default interval that is 12 wakeups per minute with no
allocation and no I/O until a hang is detected. It writes at most one `hang` report per stall; the
next `heartbeat()` re-arms it.

```rust
use quickgui::{Watchdog, WatchdogOptions};

let watchdog = Watchdog::start(&reporter, WatchdogOptions::default())?;
// Call this from the thread whose responsiveness you are observing.
watchdog.heartbeat();
```

Dropping the `Watchdog` stops and joins its thread.

### Go

`native.CrashReporter` exposes the core report store through completion callbacks.
This captures core reports; it does not install a Go `recover` handler or convert
unhandled Go panics into Rust panic reports. Go owns its own signal handlers, so
the example leaves native fatal-signal capture disabled.

```go
func StartCrashReporting() {
	captureSignals := false
	native.CrashReporter.Start(
		native.CrashReporterOptions{
			AppName:        "Demo",
			AppVersion:     "1.0.0",
			AppIdentifier:  "com.example.demo",
			MaxReports:     16,
			Parameters:     []native.CrashParameter{{Key: "channel", Value: "beta"}},
			CaptureSignals: &captureSignals,
		},
		func(directory string, err error) {
			if err != nil {
				log.Print(err)
				return
			}
			native.CrashReporter.GetLastCrashReport(func(report *native.CrashReport, err error) {
				if err != nil {
					log.Print(err)
				} else if report != nil {
					log.Print(report.Message, report.Timestamp)
				}
			})
		},
	)
}
```

`AddExtraParameter`, `RemoveExtraParameter`, and `DeleteReport` return their result
through a `func(bool, error)` callback. `UploadPending(endpoint, done)` uploads only
when explicitly called; an empty endpoint uses the configured endpoint.
`IsStarted()` is a synchronous flag query returning `(bool, error)`. Operations
that perform I/O run asynchronously; the watchdog is not exposed by the Go SDK.

## Process, system, and frame metrics

`ProcessMetrics::current()` and `SystemMemory::current()` are single explicit reads; neither
starts a thread or caches. `CpuUsageSampler` holds one previous reading and converts cumulative
CPU time into a percentage of one core.

```rust
use quickgui::{CpuUsageSampler, ProcessMetrics, SystemMemory};

let metrics = ProcessMetrics::current()?;
println!("{} MiB resident", metrics.resident_bytes / (1024 * 1024));

let mut sampler = CpuUsageSampler::new();
let _first = sampler.sample()?;          // percent is None: nothing to compare against yet
let usage = sampler.sample()?;           // percent of one core since the previous sample

let memory = SystemMemory::current()?;
println!("{} / {} bytes free", memory.available_bytes, memory.total_bytes);
```

| Field | macOS | Linux | Windows | Other |
| --- | --- | --- | --- | --- |
| `cpu_user`, `cpu_system` | `proc_pidinfo(PROC_PIDTASKALLINFO)` | `/proc/self/stat` fields 14–15 | `GetProcessTimes` | `Unsupported` |
| `resident_bytes` | `pti_resident_size` | `/proc/self/stat` field 24 × page size | `WorkingSetSize` | `Unsupported` |
| `footprint_bytes` | `proc_pid_rusage(RUSAGE_INFO_V2).ri_phys_footprint` | `None` | `None` | `Unsupported` |
| `virtual_bytes` | `pti_virtual_size` | `/proc/self/stat` field 23 | `PagefileUsage` | `Unsupported` |
| `thread_count` | `pti_threadnum` | `/proc/self/stat` field 20 | `None` | `Unsupported` |
| `uptime` | `pbi_start_tvsec` | `/proc/uptime` − `starttime` | `GetProcessTimes` creation time | `Unsupported` |
| `SystemMemory` | `sysctl hw.memsize` + `host_statistics64` | `/proc/meminfo` | `GlobalMemoryStatusEx` | `Unsupported` |

`footprint_bytes` and `thread_count` are `Option`: a platform that does not expose the value
reports `None` rather than a guess. `CpuUsage::percent` is `None` for the first sample and for an
interval longer than `MAX_CPU_SAMPLE_INTERVAL` (24 hours). Values are percentages of **one** core,
so a fully busy four-core process reports `400`.

```go
func ReadProcessMetrics() {
	native.Metrics.GetProcessMetrics(func(metrics native.ProcessMetrics, err error) {
		if err != nil {
			log.Print(err)
			return
		}
		log.Print("Resident bytes: ", metrics.ResidentBytes)
	})
	native.Metrics.GetSystemMemory(func(memory native.SystemMemory, err error) {
		if err != nil {
			log.Print(err)
			return
		}
		log.Print("Available bytes: ", memory.AvailableBytes)
	})
}
```

`native.NewCPUSampler()` returns a retained sampler and an error. Call `Sample()`
at application-chosen times on the application goroutine; the first sample has a
nil `Percent`. Call `Release()` when finished. Sampling creates no polling loop.
Go reports byte counts as `uint64`, duration seconds as `float64`, and optional
platform fields as pointers, preserving absence separately from a zero value.

Frame timing is retained per window by the renderer. Reading it is also explicit: it does not
request a redraw or start a polling loop. The result is `null`/`nil` before the first completed
frame. An idle window keeps its last result, so compare `frameNumber`/`FrameNumber` when a caller
needs to detect a new frame. FPS can be derived as `1000 / smoothedFrameMilliseconds`.

```ts
import { Metrics } from "@quickgui/native";

const frame = await Metrics.getFrameMetrics(window);
if (frame) console.log(1000 / frame.smoothedFrameMilliseconds);
```

```go
native.Metrics.GetFrameMetrics(window, func(frame *native.FrameMetrics, err error) {
	if err == nil && frame != nil {
		log.Print("FPS: ", 1000/frame.SmoothedFrameMilliseconds)
	}
})
```

Return to the [documentation index](README.md).
