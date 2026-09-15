//! Package-level commands reuse the generic async extension service contract.
use quickgui_extension_sdk::{WakeHandle, abi};
use serde::Deserialize;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
};

const MAX_REQUEST: usize = 64 * 1024;
const MAX_PENDING: usize = 16;
const MAX_SURFACES: usize = 4096;
static WAKES: Mutex<Vec<Weak<WakeHandle>>> = Mutex::new(Vec::new());
static WORKER: OnceLock<Result<Worker, String>> = OnceLock::new();

pub fn attach(wake: WakeHandle) -> Result<Arc<WakeHandle>, String> {
    let mut wakes = WAKES.lock().unwrap_or_else(|e| e.into_inner());
    wakes.retain(|w| w.strong_count() > 0);
    if wakes.len() >= MAX_SURFACES {
        return Err("too many editor surfaces".into());
    }
    let wake = Arc::new(wake);
    wakes.push(Arc::downgrade(&wake));
    Ok(wake)
}
fn invalidate() {
    let wakes: Vec<_> = WAKES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter_map(Weak::upgrade)
        .collect();
    for wake in wakes {
        wake.invalidate();
    }
}

struct Sink(abi::ServiceSink);
// The service ABI permits emit/release on a worker; ownership is transferred exactly once.
unsafe impl Send for Sink {}
impl Drop for Sink {
    fn drop(&mut self) {
        unsafe { (self.0.release)(self.0.context) };
    }
}
impl Sink {
    fn emit(&self, kind: u32, value: &[u8]) {
        unsafe { (self.0.emit)(self.0.context, kind, abi::Bytes::new(value)) };
    }
    fn fail(&self, error: impl AsRef<str>) {
        let error = error.as_ref();
        let mut end = error.len().min(16 * 1024);
        while !error.is_char_boundary(end) {
            end -= 1;
        }
        self.emit(1, &error.as_bytes()[..end]);
    }
}
struct Job {
    path: String,
    sink: Sink,
}
struct Worker {
    sender: SyncSender<Option<Job>>,
    stopped: Arc<AtomicBool>,
}
impl Worker {
    fn new() -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel::<Option<Job>>(MAX_PENDING);
        let stopped = Arc::new(AtomicBool::new(false));
        let shutdown = stopped.clone();
        std::thread::Builder::new()
            .name("quickgui-languages".into())
            .spawn(move || {
                while let Ok(Some(job)) = receiver.recv() {
                    if shutdown.load(Ordering::Acquire) {
                        job.sink.fail("editor service is shutting down");
                        break;
                    }
                    let before = crate::syntax::syntax_language_generation();
                    let result = catch_unwind(AssertUnwindSafe(|| load_pack(&job.path)));
                    match result {
                        Ok(Ok(languages)) => {
                            if crate::syntax::syntax_language_generation() != before {
                                invalidate();
                            }
                            job.sink.emit(
                                0,
                                &serde_json::to_vec(
                                    &languages
                                        .into_iter()
                                        .map(|language| language.name())
                                        .collect::<Vec<_>>(),
                                )
                                .unwrap(),
                            );
                        }
                        Ok(Err(error)) => job.sink.fail(error.to_string()),
                        Err(_) => job.sink.fail("language pack registration panicked"),
                    }
                }
                for pending in receiver.try_iter().flatten() {
                    pending.sink.fail("editor service is shutting down");
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { sender, stopped })
    }
}

unsafe fn bytes<'a>(v: abi::Bytes) -> Result<&'a [u8], String> {
    if v.len > MAX_REQUEST || (v.len > 0 && v.data.is_null()) {
        return Err("invalid language service payload".into());
    }
    Ok(if v.len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(v.data, v.len) }
    })
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    path: String,
}

unsafe extern "C" fn invoke(
    _: u32,
    method: abi::Bytes,
    params: abi::Bytes,
    sink: abi::ServiceSink,
) {
    let sink = Sink(sink);
    let request = catch_unwind(AssertUnwindSafe(|| -> Result<Request, String> {
        if unsafe { bytes(method)? } != b"load-language-pack" {
            return Err("unknown editor service method".into());
        }
        let request: Request =
            serde_json::from_slice(unsafe { bytes(params)? }).map_err(|e| e.to_string())?;
        if request.path.len() > 4096 || !Path::new(&request.path).is_absolute() {
            return Err("language pack path must be an absolute path of at most 4096 bytes".into());
        }
        Ok(request)
    }));
    let request = match request {
        Ok(Ok(request)) => request,
        Ok(Err(error)) => {
            sink.fail(error);
            return;
        }
        Err(_) => {
            sink.fail("invalid language service request");
            return;
        }
    };
    let worker = match WORKER.get_or_init(Worker::new) {
        Ok(worker) => worker,
        Err(error) => {
            sink.fail(error);
            return;
        }
    };
    if worker.stopped.load(Ordering::Acquire) {
        sink.fail("editor service is shutting down");
        return;
    }
    if let Err(error) = worker.sender.try_send(Some(Job {
        path: request.path,
        sink,
    })) {
        let (TrySendError::Full(pending) | TrySendError::Disconnected(pending)) = error;
        if let Some(job) = pending {
            job.sink
                .fail("language registration queue is full or closed");
        }
    }
}
unsafe extern "C" fn shutdown() {
    if let Some(Ok(worker)) = WORKER.get()
        && !worker.stopped.swap(true, Ordering::AcqRel)
    {
        let _ = worker.sender.try_send(None);
    }
}
pub const API: abi::ServiceApi = abi::ServiceApi { invoke, shutdown };

#[cfg(feature = "language-packs")]
fn load_pack(path: &str) -> Result<Vec<crate::SyntaxLanguage>, String> {
    crate::syntax::load_syntax_language_pack(path).map_err(|e| e.to_string())
}
#[cfg(not(feature = "language-packs"))]
fn load_pack(_: &str) -> Result<Vec<crate::SyntaxLanguage>, String> {
    Err("this editor library was built without language-packs support".into())
}
