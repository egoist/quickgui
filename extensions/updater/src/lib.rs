//! Optional automatic updater. The extension owns network, signing and installer code; it
//! communicates with the existing host event queue through a bounded C ABI, never a Go thread.
#![allow(dead_code)] // The same ABI source also declares terminal-only layouts.

#[path = "../../../src/extension_api.rs"]
mod abi;
mod feed;
#[cfg(any(not(target_os = "macos"), test))]
#[path = "handoff.rs"]
pub mod handoff;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(not(target_os = "macos"), test))]
mod portable;

use abi::{Bytes, ServiceSink};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(any(target_os = "macos", test))]
use std::ffi::c_void;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
struct Options {
    feed_url: String,
    public_key: String,
    current_version: String,
    identifier: String,
    automatic_checks: bool,
    development: bool,
    allow_development: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            feed_url: String::new(),
            public_key: String::new(),
            current_version: String::new(),
            identifier: String::new(),
            automatic_checks: true,
            development: true,
            allow_development: false,
        }
    }
}
impl Options {
    fn disabled(&self) -> bool {
        self.development && !self.allow_development
    }
    fn validate(&self) -> Result<()> {
        if self.disabled() {
            return Ok(());
        }
        feed::https_url(&self.feed_url)?;
        feed::verifying_key(&self.public_key)?;
        semver::Version::parse(&self.current_version)
            .map_err(|_| "updater currentVersion must be a semantic version")?;
        if self.identifier.is_empty()
            || matches!(self.identifier.as_str(), "." | "..")
            || self.identifier.len() > 255
            || !self
                .identifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        {
            return Err("updater identifier must contain only letters, numbers, dots, underscores and hyphens".into());
        }
        Ok(())
    }
}

struct Sink(ServiceSink);
// SAFETY: The host's sink explicitly supports native worker threads; release only runs when the
// last Arc drops. No borrowed inputs or Objective-C objects are sent through this wrapper.
unsafe impl Send for Sink {}
unsafe impl Sync for Sink {}
impl Sink {
    fn emit(&self, kind: u32, value: &[u8]) {
        unsafe { (self.0.emit)(self.0.context, kind, Bytes::new(value)) };
    }
    fn reply(&self) {
        self.emit(0, b"null");
    }
    fn error(&self, error: impl AsRef<str>) {
        let s = error.as_ref();
        self.emit(1, &s.as_bytes()[..s.len().min(16 * 1024)]);
    }
    fn event(&self, value: &Event) {
        if let Ok(bytes) = serde_json::to_vec(value) {
            self.emit(2, &bytes);
        }
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    Available,
    Downloading,
    #[serde(rename = "installing")]
    Updating,
    Disabled,
}
enum UpdaterEvent {
    StatusChanged(UpdateStatus),
    UpToDate,
    Failed(String),
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Event {
    kind: String,
    status: UpdateStatus,
    #[serde(skip_serializing_if = "String::is_empty")]
    version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    notes: String,
    automatic_checks: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
    #[serde(skip_serializing_if = "is_zero")]
    downloaded_bytes: u64,
    #[serde(skip_serializing_if = "is_zero")]
    total_bytes: u64,
    quit_required: bool,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
struct EventsInner {
    sink: Arc<Sink>,
    state: Mutex<Event>,
    closed: AtomicBool,
}
#[derive(Clone)]
struct Events(Arc<EventsInner>);
impl Events {
    fn new(sink: Arc<Sink>, automatic: bool) -> Self {
        Self(Arc::new(EventsInner {
            sink,
            state: Mutex::new(Event {
                automatic_checks: automatic,
                ..Default::default()
            }),
            closed: AtomicBool::new(false),
        }))
    }
    fn stopped(&self) -> bool {
        self.0.closed.load(Ordering::Acquire)
    }
    fn close(&self) {
        self.0.closed.store(true, Ordering::Release);
    }
    fn update(&self, kind: &str, change: impl FnOnce(&mut Event)) {
        if self.stopped() {
            return;
        }
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.kind = kind.into();
        state.error.clear();
        change(&mut state);
        if !self.stopped() {
            self.0.sink.event(&state);
        }
    }
    fn try_send(&self, event: UpdaterEvent) {
        match event {
            UpdaterEvent::StatusChanged(status) => self.update("state", |e| e.status = status),
            UpdaterEvent::UpToDate => self.update("up-to-date", |e| e.status = UpdateStatus::Idle),
            UpdaterEvent::Failed(error) => self.update("error", |e| {
                e.status = UpdateStatus::Idle;
                e.error = error;
            }),
        }
    }
}

#[derive(Deserialize)]
struct Command {
    session: u32,
    #[serde(default)]
    value: Value,
}

unsafe fn copy_bytes<'a>(bytes: Bytes, maximum: usize) -> Result<&'a str> {
    if bytes.len > maximum || bytes.data.is_null() {
        return Err("invalid updater request span".into());
    }
    std::str::from_utf8(unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) })
        .map_err(|_| "updater request must be UTF-8".into())
}
unsafe extern "C" fn invoke(request: u32, method: Bytes, params: Bytes, callback: ServiceSink) {
    let sink = Arc::new(Sink(callback));
    let input = unsafe {
        copy_bytes(method, 64)
            .and_then(|m| copy_bytes(params, 64 * 1024).map(|p| (m.to_owned(), p.to_owned())))
    };
    let (method, params) = match input {
        Ok(v) => v,
        Err(e) => {
            sink.error(e);
            return;
        }
    };
    schedule(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            #[cfg(target_os = "macos")]
            {
                macos::invoke(request, &method, &params, sink.clone())
            }
            #[cfg(not(target_os = "macos"))]
            {
                portable::invoke(request, &method, &params, sink.clone())
            }
        }))
        .unwrap_or_else(|_| Err("updater encountered an internal error".into()));
        if let Err(error) = result {
            sink.error(error);
        }
    });
}
unsafe extern "C" fn shutdown() {
    #[cfg(target_os = "macos")]
    if objc2::MainThreadMarker::new().is_some() {
        // The host is leaving its main loop; queued cleanup would no longer be pumped.
        macos::shutdown();
        return;
    }
    schedule(|| {
        #[cfg(target_os = "macos")]
        macos::shutdown();
        #[cfg(not(target_os = "macos"))]
        portable::shutdown();
    });
}

#[cfg(target_os = "macos")]
fn schedule(task: impl FnOnce() + Send + 'static) {
    unsafe extern "C" {
        fn dispatch_async_f(
            queue: *mut c_void,
            context: *mut c_void,
            work: unsafe extern "C" fn(*mut c_void),
        );
        static _dispatch_main_q: c_void;
    }
    unsafe extern "C" fn run(context: *mut c_void) {
        let task = unsafe { Box::from_raw(context.cast::<Box<dyn FnOnce() + Send>>()) };
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task));
    }
    let task: Box<Box<dyn FnOnce() + Send>> = Box::new(Box::new(task));
    unsafe {
        dispatch_async_f(
            std::ptr::addr_of!(_dispatch_main_q).cast_mut(),
            Box::into_raw(task).cast(),
            run,
        );
    }
}
#[cfg(not(target_os = "macos"))]
fn schedule(task: impl FnOnce() + Send + 'static) {
    type Task = Box<dyn FnOnce() + Send>;
    static QUEUE: std::sync::OnceLock<std::sync::mpsc::SyncSender<Task>> =
        std::sync::OnceLock::new();
    let queue = QUEUE.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Task>(128);
        let _ = std::thread::Builder::new()
            .name("quickgui-updater-commands".into())
            .spawn(move || {
                for task in rx {
                    task();
                }
            });
        tx
    });
    // A full queue drops the task and releases its sink with a host error; never block the UI.
    let _ = queue.try_send(Box::new(task));
}

static API: abi::ServiceApi = abi::ServiceApi { invoke, shutdown };
static EXTENSION: abi::Extension = abi::Extension {
    abi_version: abi::ABI_VERSION,
    descriptor_size: size_of::<abi::Extension>() as u32,
    kind: abi::SERVICE_EXTENSION,
    api_size: size_of::<abi::ServiceApi>() as u32,
    name: Bytes::new(b"updater"),
    version: Bytes::new(env!("CARGO_PKG_VERSION").as_bytes()),
    api: std::ptr::addr_of!(API).cast(),
};
/// # Safety
/// The returned descriptor and function table must only be used according to the extension ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn quickgui_extension_v1() -> *const abi::Extension {
    &EXTENSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    #[derive(Default)]
    struct Capture {
        messages: Mutex<Vec<(u32, Vec<u8>)>>,
        releases: AtomicUsize,
    }
    unsafe extern "C" fn record(context: *mut c_void, kind: u32, bytes: Bytes) {
        let capture = unsafe { &*context.cast::<Arc<Capture>>() };
        capture.messages.lock().unwrap().push((
            kind,
            unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) }.to_vec(),
        ));
    }
    unsafe extern "C" fn release(context: *mut c_void) {
        let capture = unsafe { Box::from_raw(context.cast::<Arc<Capture>>()) };
        capture.releases.fetch_add(1, Ordering::Relaxed);
    }
    fn sink(capture: &Arc<Capture>) -> Arc<Sink> {
        Arc::new(Sink(ServiceSink {
            context: Box::into_raw(Box::new(capture.clone())).cast(),
            emit: record,
            release,
        }))
    }
    #[test]
    fn disabled_session_retains_stream_then_releases_it_once_on_stop() {
        let capture = Arc::new(Capture::default());
        #[cfg(target_os = "macos")]
        use crate::macos as platform;
        #[cfg(not(target_os = "macos"))]
        use crate::portable as platform;
        platform::invoke(900, "start", r#"{"development":true}"#, sink(&capture)).unwrap();
        assert_eq!(capture.releases.load(Ordering::Relaxed), 0);
        let messages = capture.messages.lock().unwrap();
        assert!(
            messages.iter().any(
                |(kind, data)| *kind == 2 && String::from_utf8_lossy(data).contains("disabled")
            )
        );
        assert_eq!(messages.iter().filter(|(kind, _)| *kind == 0).count(), 1);
        drop(messages);
        let stop = Arc::new(Capture::default());
        platform::invoke(901, "stop", r#"{"session":900}"#, sink(&stop)).unwrap();
        assert_eq!(capture.releases.load(Ordering::Relaxed), 1);
        assert_eq!(stop.releases.load(Ordering::Relaxed), 1);
    }
    #[test]
    fn malformed_abi_input_releases_context_even_before_dispatch() {
        let capture = Arc::new(Capture::default());
        let callback = ServiceSink {
            context: Box::into_raw(Box::new(capture.clone())).cast(),
            emit: record,
            release,
        };
        unsafe {
            invoke(
                1,
                Bytes {
                    data: std::ptr::null(),
                    len: 1,
                },
                Bytes::new(b"{}"),
                callback,
            )
        };
        assert_eq!(capture.releases.load(Ordering::Relaxed), 1);
        assert_eq!(capture.messages.lock().unwrap()[0].0, 1);
    }
}
