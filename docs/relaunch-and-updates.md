# Relaunch and signed updates

[Documentation index](README.md)

QuickGUI owns relaunch scheduling in the Rust application core. Calling `cx.relaunch()` prepares
the replacement process immediately, requests the ordinary child-first application teardown, and
spawns only after native windows and callbacks are complete, foreground work is cancelled,
background queues are closed, and global shortcuts, tray icons, and the single-instance guard have
been released:

```rust
use quickgui::EventContext;

fn restart(cx: &mut EventContext) -> Result<(), quickgui::SystemIntegrationError> {
    cx.relaunch()
}
```

`RelaunchOptions` can replace the executable, arguments, or working directory. Unspecified values
preserve the current process. Paths must be absolute, all native strings are NUL-free and bounded,
and the replacement is launched directly without a shell. One application retains at most one
request. `AppRunner::relaunch` follows the same boundary; its next exit-producing `pump` releases
process services, spawns once, and exposes the resulting process ID through
`relaunched_process()`.

`on_before_quit` and `on_will_quit` receive a `QuitRequest` whose reason distinguishes an explicit
quit, operating-system termination, final-window policy, and relaunch. Calling `prevent_quit()` in
either phase cancels that attempt; cancelling a relaunch also discards its prepared replacement
process. macOS answers `applicationShouldTerminate` only after both Rust callbacks complete, so the
native application and core lifecycle cannot disagree.

The deterministic `TestAppContext` performs the complete window/callback teardown but never
spawns. `relaunch_request()` exposes the prepared value for assertions.

## Signed updates

Automatic updates are documented in the [updater guide](updater.md): Rust applications enable the
core `updater` feature, and Go and TypeScript applications load the same implementation as a
native extension. The install helper relaunches an updated application itself; `cx.relaunch()` is
not part of that flow.
