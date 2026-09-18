//! Windows/Linux: the same Sparkle appcast and Ed25519 signature, with application-owned UI.
use super::{
    Command, Result, SessionEvents as Events, SessionOptions as Options, Sink, UpdateStatus,
    UpdaterEvent, feed, handoff,
};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Session {
    id: u32,
    options: Options,
    events: Events,
    available: Mutex<Option<feed::Item>>,
    operation: Mutex<Operation>,
    preferences: Option<PathBuf>,
}
#[derive(Clone, Copy, Default)]
enum Operation {
    #[default]
    Idle,
    Checking {
        explicit: bool,
    },
    Installing,
}

static SESSION: Mutex<Option<Arc<Session>>> = Mutex::new(None);

pub(super) fn invoke(id: u32, method: &str, params: &str, sink: Arc<Sink>) -> Result<()> {
    if method == "start" {
        let options: Options = serde_json::from_str(params).map_err(|e| e.to_string())?;
        options.validate()?;
        let mut slot = SESSION.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return Err("an updater is already running; share one per application".into());
        }
        let preferences = if options.disabled() {
            None
        } else {
            Some(preference_path(&options.identifier)?)
        };
        let automatic = preferences
            .as_ref()
            .map(|p| read_preference(p, options.automatic_checks))
            .unwrap_or(options.automatic_checks);
        let events = Events::new(sink.clone(), automatic);
        // A package-managed install can still initialize and expose a Disabled state. It must
        // never offer an update it cannot safely install, or mutate /usr/bin or a mounted image.
        let unsupported = if options.disabled() {
            Some("updating is disabled in development builds".into())
        } else {
            handoff::install_target().err()
        };
        let session = Arc::new(Session {
            id,
            options,
            events,
            available: Mutex::new(None),
            operation: Mutex::new(Operation::Idle),
            preferences,
        });
        session.events.update("state", |e| {
            e.status = if unsupported.is_some() {
                UpdateStatus::Disabled
            } else {
                UpdateStatus::Idle
            };
            e.error = unsupported.clone().unwrap_or_default();
        });
        *slot = Some(session.clone());
        drop(slot);
        sink.reply();
        if unsupported.is_none() && automatic {
            session.check(false)?;
        }
        return Ok(());
    }
    let command: Command = serde_json::from_str(params).map_err(|e| e.to_string())?;
    let mut slot = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    let session = slot
        .as_ref()
        .filter(|s| s.id == command.session)
        .cloned()
        .ok_or("updater session has been closed")?;
    if method == "stop" {
        session.events.close();
        slot.take();
        sink.reply();
        return Ok(());
    }
    drop(slot);
    if session.options.disabled() {
        return Err("updating is disabled in development builds".into());
    }
    handoff::install_target()?;
    match method {
        "check" => {
            session.check(true)?;
            sink.reply();
        }
        "install" => session.install(sink)?,
        "automatic" => {
            let enabled = command
                .value
                .as_bool()
                .ok_or("automatic expects a boolean")?;
            let path = session
                .preferences
                .as_ref()
                .ok_or("updater preferences unavailable")?;
            // Commands run on background workers; serialize persistence with the event state so
            // rapid preference changes cannot leave an older async write on disk.
            let mut state = session
                .events
                .0
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            write_preference(path, enabled)?;
            state.automatic_checks = enabled;
            drop(state);
            session.events.update("state", |_| {});
            sink.reply();
            if enabled {
                session.check(false)?;
            }
        }
        _ => return Err(format!("unknown updater command: {method}")),
    }
    Ok(())
}

#[allow(dead_code)] // See `super::shutdown`.
pub(super) fn shutdown() {
    if let Some(session) = SESSION.lock().unwrap_or_else(|e| e.into_inner()).take() {
        session.events.close();
    }
}

impl Session {
    fn check(self: &Arc<Self>, explicit: bool) -> Result<()> {
        let mut operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
        match &mut *operation {
            Operation::Checking { explicit: previous } => {
                *previous |= explicit;
                return Ok(());
            }
            Operation::Installing => return Err("an update is being installed".into()),
            Operation::Idle => *operation = Operation::Checking { explicit },
        }
        drop(operation);
        if explicit {
            self.events
                .try_send(UpdaterEvent::StatusChanged(UpdateStatus::Checking));
        }
        let session = self.clone();
        let result = std::thread::Builder::new()
            .name("quickgui-updater-check".into())
            .spawn(move || {
                let result = (|| {
                    let bytes = fetch(
                        &session.options.feed_url,
                        feed::MAX_FEED_BYTES as u64,
                        &session.events,
                        |_, _| {},
                    )?;
                    let xml = std::str::from_utf8(&bytes).map_err(|_| "appcast must be UTF-8")?;
                    feed::newest(
                        xml,
                        &session.options.current_version,
                        std::env::consts::OS,
                        handoff::install_target()?.payload,
                    )
                })();
                let mut operation = session.operation.lock().unwrap_or_else(|e| e.into_inner());
                let report = matches!(*operation, Operation::Checking { explicit: true });
                match result {
                    Ok(Some(item)) => {
                        *session.available.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(item.clone());
                        session.events.update("state", |e| {
                            e.status = UpdateStatus::Available;
                            e.version = item.version;
                            e.notes = item.notes;
                            e.downloaded_bytes = 0;
                            e.total_bytes = item.length;
                        });
                    }
                    Ok(None) => {
                        *session.available.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        if report {
                            session.events.try_send(UpdaterEvent::UpToDate);
                        }
                    }
                    Err(error) => session.events.try_send(UpdaterEvent::Failed(error)),
                }
                *operation = Operation::Idle;
            });
        if result.is_err() {
            *self.operation.lock().unwrap_or_else(|e| e.into_inner()) = Operation::Idle;
            return Err("could not start updater check".into());
        }
        Ok(())
    }
    fn install(self: &Arc<Self>, sink: Arc<Sink>) -> Result<()> {
        let mut operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(*operation, Operation::Idle) {
            return Err("an update operation is already running".into());
        }
        *operation = Operation::Installing;
        drop(operation);
        let item = self
            .available
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let Some(item) = item else {
            *self.operation.lock().unwrap_or_else(|e| e.into_inner()) = Operation::Idle;
            return Err("no available update; check first".into());
        };
        let stage_root = self
            .preferences
            .as_ref()
            .and_then(|p| p.parent())
            .ok_or("updater preferences unavailable")?
            .join("updater-stages");
        let session = self.clone();
        let result = std::thread::Builder::new()
            .name("quickgui-updater-install".into())
            .spawn(move || {
                let result = (|| {
                    session
                        .events
                        .try_send(UpdaterEvent::StatusChanged(UpdateStatus::Downloading));
                    let mut last = Instant::now();
                    let bytes = fetch(
                        &item.url,
                        item.length,
                        &session.events,
                        |downloaded, total| {
                            if downloaded == total || last.elapsed() >= Duration::from_millis(100) {
                                last = Instant::now();
                                session.events.update("progress", |e| {
                                    e.downloaded_bytes = downloaded;
                                    e.total_bytes = total;
                                });
                            }
                        },
                    )?;
                    feed::verify(&bytes, &item, &session.options.public_key)?;
                    if session.events.stopped() {
                        return Err("update cancelled".into());
                    }
                    handoff::launch(
                        bytes,
                        &item,
                        &session.options.public_key,
                        &stage_root,
                        || session.events.stopped(),
                        || {
                            session.events.update("state", |e| {
                                e.status = UpdateStatus::Installing;
                                e.quit_required = true;
                            });
                            sink.reply();
                        },
                    )
                })();
                if let Err(error) = result {
                    sink.error(&error);
                    session.events.update("error", |e| {
                        e.status = UpdateStatus::Available;
                        e.quit_required = false;
                        e.error = error;
                    });
                }
                *session.operation.lock().unwrap_or_else(|e| e.into_inner()) = Operation::Idle;
            });
        if result.is_err() {
            *self.operation.lock().unwrap_or_else(|e| e.into_inner()) = Operation::Idle;
            return Err("could not start update installation".into());
        }
        Ok(())
    }
}

fn fetch(
    url: &str,
    maximum: u64,
    events: &Events,
    mut progress: impl FnMut(u64, u64),
) -> Result<Vec<u8>> {
    feed::https_url(url)?;
    let config = ureq::Agent::config_builder()
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(600)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_body(Some(Duration::from_secs(30)))
        .max_redirects(5)
        .build();
    let mut response = ureq::Agent::new_with_config(config)
        .get(url)
        .call()
        .map_err(|e| e.to_string())?;
    if response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .is_some_and(|n| n > maximum)
    {
        return Err("update response exceeds its size limit".into());
    }
    let mut reader = response.body_mut().as_reader();
    let mut bytes = Vec::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        if events.stopped() {
            return Err("update cancelled".into());
        }
        let n = reader.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if bytes.len() as u64 + n as u64 > maximum {
            return Err("update response exceeds its size limit".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
        progress(bytes.len() as u64, maximum);
    }
    Ok(bytes)
}

fn preference_path(id: &str) -> Result<PathBuf> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let root = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/state")));
    Ok(root
        .filter(|p| p.is_absolute())
        .ok_or("could not locate per-user update preferences")?
        .join(id)
        .join("updater.json"))
}
fn read_preference(path: &PathBuf, default: bool) -> bool {
    fs::File::open(path)
        .ok()
        .and_then(|f| {
            let mut bytes = Vec::new();
            f.take(1025).read_to_end(&mut bytes).ok()?;
            if bytes.len() > 1024 {
                return None;
            }
            serde_json::from_slice::<bool>(&bytes).ok()
        })
        .unwrap_or(default)
}
fn write_preference(path: &PathBuf, value: bool) -> Result<()> {
    let parent = path.parent().ok_or("invalid preference path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(if value { b"true" } else { b"false" })
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_are_bounded_and_persist_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app/updater.json");
        assert!(read_preference(&path, true));
        write_preference(&path, false).unwrap();
        assert!(!read_preference(&path, true));
        write_preference(&path, true).unwrap();
        assert!(read_preference(&path, false));
        fs::write(&path, "x".repeat(1025)).unwrap();
        assert!(!read_preference(&path, false));
    }
}
