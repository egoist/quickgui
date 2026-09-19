//! Rust applications link this crate, so they drive the platform sessions directly instead of
//! loading a library. The session protocol stays the one Go and TypeScript use: commands and
//! events keep a single tested shape on every platform, including Sparkle's main-thread driver.
use super::{Event, Result, Sink, abi, dispatch};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::{
    collections::VecDeque,
    ffi::c_void,
    fmt,
    future::poll_fn,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU32, Ordering},
    },
    task::{Poll, Waker},
};

/// Transient events (`up-to-date`, `error`) queue here until the application reads them. A
/// reader that falls this far behind loses the oldest; [`Updater::state`] always stays current.
const MAX_PENDING_EVENTS: usize = 64;
/// Request IDs the native host hands to Go and TypeScript sessions are never seen here.
static NEXT_REQUEST: AtomicU32 = AtomicU32::new(1);
#[cfg(test)]
pub(crate) static TEST_SESSION: Mutex<()> = Mutex::new(());

/// A rejected command or an updater that could not start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

/// Overrides for the defaults `quickgui build` embeds from `[updates]`. Development builds stay
/// disabled unless `allow_development` is set for update testing. `current_version` and
/// `identifier` apply to Windows and Linux; Sparkle reads them from the app bundle.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub allow_development: bool,
    #[serde(skip)]
    metadata: Option<&'static str>,
}
impl Options {
    /// Use [`updater_options!`](crate::updater_options), which reads the value in the application
    /// crate so Cargo rebuilds it when `[updates]` changes.
    #[doc(hidden)]
    pub fn from_build_metadata(metadata: Option<&'static str>) -> Self {
        Self {
            metadata,
            ..Self::default()
        }
    }
    fn session_parameters(&self) -> Result<String> {
        let mut parameters = Map::new();
        if let Some(metadata) = self.metadata.filter(|metadata| !metadata.is_empty()) {
            use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
            parameters = URL_SAFE_NO_PAD
                .decode(metadata)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .ok_or("invalid updater build metadata")?;
        }
        if let Ok(Value::Object(overrides)) = serde_json::to_value(self) {
            parameters.extend(overrides);
        }
        Ok(Value::Object(parameters).to_string())
    }
}

/// The updater settings `quickgui build` embedded for this application. Without the CLI the
/// result is empty and the updater reports [`UpdateStatus::Disabled`](super::UpdateStatus).
#[macro_export]
macro_rules! updater_options {
    () => {
        $crate::updater::Options::from_build_metadata(::core::option_env!(
            "QUICKGUI_UPDATER_METADATA"
        ))
    };
}

/// Tell the install helper that the updated application started. `quickgui` calls this when the
/// application becomes ready; without it the helper restores the previous version.
pub fn acknowledge_startup() {
    let Some(path) = std::env::var_os(READY_ENV).map(std::path::PathBuf::from) else {
        return;
    };
    // SAFETY: Runs once on the main thread during startup, before the application starts workers
    // that read the environment; children must not inherit the helper's private path.
    unsafe { std::env::remove_var(READY_ENV) };
    if path.is_absolute()
        && path.file_name() == Some("application-ready".as_ref())
        && path.as_os_str().len() < 4096
    {
        let _ = std::thread::Builder::new()
            .name("quickgui-updater-ready".into())
            .spawn(move || {
                use std::io::Write;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    let _ = file.write_all(b"ready\n");
                }
            });
    }
}
const READY_ENV: &str = "QUICKGUI_UPDATE_READY_FILE";

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// One reply (or rejection) per command, awaited without blocking the application thread.
#[derive(Default)]
struct Reply {
    result: Option<std::result::Result<(), Error>>,
    waker: Option<Waker>,
}
type SharedReply = Arc<Mutex<Reply>>;
fn complete(reply: &SharedReply, result: std::result::Result<(), Error>) {
    let mut reply = lock(reply);
    if reply.result.is_none() {
        reply.result = Some(result);
        if let Some(waker) = reply.waker.take() {
            waker.wake();
        }
    }
}
async fn wait(reply: SharedReply) -> std::result::Result<(), Error> {
    poll_fn(|cx| {
        let mut reply = lock(&reply);
        match reply.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                reply.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    })
    .await
}

#[derive(Default)]
struct Stream {
    state: Event,
    pending: VecDeque<Event>,
    waker: Option<Waker>,
    closed: bool,
}

/// What a service sink calls back into. Dropping it is the sink's single `release`.
struct Receiver {
    reply: SharedReply,
    stream: Option<Arc<Mutex<Stream>>>,
}
impl Receiver {
    fn sink(self) -> Arc<Sink> {
        unsafe extern "C" fn emit(context: *mut c_void, kind: u32, bytes: abi::Bytes) {
            let receiver = unsafe { &*context.cast::<Receiver>() };
            let bytes = unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) };
            match kind {
                0 => complete(&receiver.reply, Ok(())),
                1 => complete(
                    &receiver.reply,
                    Err(Error(String::from_utf8_lossy(bytes).into_owned())),
                ),
                _ => {
                    let (Some(stream), Ok(event)) =
                        (&receiver.stream, serde_json::from_slice::<Event>(bytes))
                    else {
                        return;
                    };
                    let mut stream = lock(stream);
                    stream.state = event.clone();
                    if stream.pending.len() == MAX_PENDING_EVENTS {
                        stream.pending.pop_front();
                    }
                    stream.pending.push_back(event);
                    if let Some(waker) = stream.waker.take() {
                        waker.wake();
                    }
                }
            }
        }
        unsafe extern "C" fn release(context: *mut c_void) {
            drop(unsafe { Box::from_raw(context.cast::<Receiver>()) });
        }
        Arc::new(Sink(abi::ServiceSink {
            context: Box::into_raw(Box::new(self)).cast(),
            emit,
            release,
        }))
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        // A full command queue releases the sink without replying.
        complete(&self.reply, Err(Error("the updater is busy".into())));
        if let Some(stream) = &self.stream {
            let mut stream = lock(stream);
            stream.closed = true;
            if let Some(waker) = stream.waker.take() {
                waker.wake();
            }
        }
    }
}

/// The application's updater. It is app-wide, not window-owned: start at most one, after the
/// application is ready, and share it between settings, menus, and any update banner. Dropping
/// it cancels checks and downloads; an accepted installer handoff already belongs to the helper.
pub struct Updater {
    session: u32,
    stream: Arc<Mutex<Stream>>,
}
impl Updater {
    /// Start native updating with the defaults from [`updater_options!`](crate::updater_options)
    /// and per-user preferences. Automatic checks begin right away when they are enabled.
    pub async fn start(options: Options) -> std::result::Result<(Self, Events), Error> {
        let parameters = options.session_parameters().map_err(Error)?;
        let session = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        let stream = Arc::new(Mutex::new(Stream::default()));
        let reply = SharedReply::default();
        dispatch(
            session,
            "start".into(),
            parameters,
            Receiver {
                reply: reply.clone(),
                stream: Some(stream.clone()),
            }
            .sink(),
        );
        wait(reply).await?;
        Ok((
            Self {
                session,
                stream: stream.clone(),
            },
            Events(stream),
        ))
    }
    /// The latest event, without native calls or polling.
    pub fn state(&self) -> Event {
        lock(&self.stream).state.clone()
    }
    /// Run a user-initiated check. macOS presents Sparkle's standard windows; elsewhere the
    /// result arrives as an `available` state or an `up-to-date` event.
    pub async fn check(&self) -> std::result::Result<(), Error> {
        self.command("check", Value::Null).await
    }
    /// Accept the offered update. macOS hands over to Sparkle; other platforms verify and stage
    /// it, then report `quit_required` once their helper waits for the application to quit.
    pub async fn install(&self) -> std::result::Result<(), Error> {
        self.command("install", Value::Null).await
    }
    /// Persist the automatic-check preference. Enabling it starts a quiet check.
    pub async fn set_automatic_checks(&self, enabled: bool) -> std::result::Result<(), Error> {
        self.command("automatic", Value::Bool(enabled)).await
    }
    fn command(
        &self,
        method: &str,
        value: Value,
    ) -> impl Future<Output = std::result::Result<(), Error>> + use<> {
        let reply = SharedReply::default();
        dispatch(
            NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
            method.into(),
            json!({ "session": self.session, "value": value }).to_string(),
            Receiver {
                reply: reply.clone(),
                stream: None,
            }
            .sink(),
        );
        wait(reply)
    }
}
impl Drop for Updater {
    fn drop(&mut self) {
        drop(self.command("stop", Value::Null));
    }
}

/// Events in order, for one reader. Await it from the application-thread executor
/// (`cx.spawn`) and update views from there.
pub struct Events(Arc<Mutex<Stream>>);
impl Events {
    /// The next event, or `None` once the updater has stopped.
    pub async fn next(&mut self) -> Option<Event> {
        poll_fn(|cx| {
            let mut stream = lock(&self.0);
            match stream.pending.pop_front() {
                Some(event) => Poll::Ready(Some(event)),
                None if stream.closed => Poll::Ready(None),
                None => {
                    stream.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await
    }
}

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::super::UpdateStatus;
    use super::*;
    use std::{pin::pin, task::Context, thread::Thread};

    struct Unpark(Thread);
    impl std::task::Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    fn block_on<T>(future: impl Future<Output = T>) -> T {
        let waker = Arc::new(Unpark(std::thread::current())).into();
        let mut future = pin!(future);
        loop {
            match future.as_mut().poll(&mut Context::from_waker(&waker)) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::park(),
            }
        }
    }

    #[test]
    fn build_metadata_and_overrides_become_session_parameters() {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let metadata = URL_SAFE_NO_PAD.encode(
            r#"{"feedUrl":"https://example.com/appcast.xml","currentVersion":"1.0.0","development":false}"#,
        );
        let mut options = Options::from_build_metadata(Some(Box::leak(metadata.into_boxed_str())));
        options.current_version = Some("2.0.0".into());
        let parameters: Value =
            serde_json::from_str(&options.session_parameters().unwrap()).unwrap();
        assert_eq!(
            parameters,
            json!({"feedUrl": "https://example.com/appcast.xml", "currentVersion": "2.0.0", "development": false})
        );
        assert_eq!(Options::default().session_parameters().unwrap(), "{}");
        assert!(
            Options::from_build_metadata(Some("not base64!"))
                .session_parameters()
                .is_err()
        );
    }

    #[test]
    fn a_development_build_starts_disabled_and_stops_with_its_handle() {
        let _session = lock(&TEST_SESSION);
        let (updater, mut events) = block_on(Updater::start(Options::default())).unwrap();
        let event = block_on(events.next()).unwrap();
        assert_eq!(event.status, UpdateStatus::Disabled);
        assert!(event.error.contains("development"));
        assert_eq!(updater.state(), event);
        let error = block_on(updater.check()).unwrap_err();
        assert!(error.to_string().contains("development"), "{error}");
        drop(updater);
        assert_eq!(block_on(events.next()), None);
        // Stopping released the single session, so the application may start another.
        let (again, _events) = block_on(Updater::start(Options::default())).unwrap();
        drop(again);
        block_on(_events_closed(_events));
    }
    async fn _events_closed(mut events: Events) {
        while events.next().await.is_some() {}
    }
}
