//! Out-of-process installation is limited to this helper: application UI and extension calls
//! stay in-process. Windows cannot replace loaded DLLs; Linux AppImage mounts and install
//! prefixes must survive until the old process exits. The helper re-verifies bytes and waits for the normal quit lifecycle.
use super::feed::{self, Item, Payload};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, String>;
const QUIT_TIMEOUT: Duration = Duration::from_secs(120);
const READY_TIMEOUT: Duration = Duration::from_secs(60);
pub const READY_ENV: &str = "QUICKGUI_UPDATE_READY_FILE";

/// What the helper replaces, and the appcast enclosure that can replace it.
pub struct Target {
    pub path: PathBuf,
    pub payload: Payload,
}

pub fn install_target() -> Result<Target> {
    #[cfg(target_os = "linux")]
    {
        if unsafe { libc::geteuid() } == 0 {
            return Err("self-updates are disabled for root installations".into());
        }
        let executable = std::env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|e| e.to_string())?;
        // A terminal or launcher that is itself an AppImage leaks both variables to its children,
        // so they only count when the mount really holds this executable.
        let mounted = std::env::var_os("APPDIR")
            .and_then(|appdir| fs::canonicalize(appdir).ok())
            .is_some_and(|mount| executable.starts_with(mount));
        let Some(appimage) = std::env::var_os("APPIMAGE").filter(|_| mounted) else {
            return super::prefix::locate(&executable)
                .map(|path| Target {
                    path,
                    payload: Payload::Prefix,
                })
                .map_err(|error| {
                    format!(
                        "this installation is managed externally ({error}); install the AppImage or run install.sh to use automatic updates"
                    )
                });
        };
        let target = fs::canonicalize(appimage).map_err(|e| e.to_string())?;
        let metadata = fs::metadata(&target).map_err(|e| e.to_string())?;
        use std::os::unix::fs::MetadataExt;
        if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } {
            return Err("AppImage must be owned by the current user".into());
        }
        let parent = target.parent().ok_or("invalid AppImage location")?;
        tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| "AppImage directory must be writable")?;
        Ok(Target {
            path: target,
            payload: Payload::AppImage,
        })
    }
    #[cfg(windows)]
    {
        std::env::current_exe()
            .and_then(fs::canonicalize)
            .map(|path| Target {
                path,
                payload: Payload::Any,
            })
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        Err("portable updater is only supported on Linux and Windows".into())
    }
}

#[derive(Serialize, Deserialize)]
struct Plan {
    parent: u32,
    target: PathBuf,
    kind: Payload,
    payload: PathBuf,
    item: Item,
    public_key: String,
}

pub(crate) fn launch(
    bytes: Vec<u8>,
    item: &Item,
    key: &str,
    stage_root: &Path,
    cancelled: impl Fn() -> bool,
    ready: impl FnOnce(),
) -> Result<()> {
    let Target {
        path: target,
        payload: kind,
    } = install_target()?;
    validate_format(&bytes, kind, &target)?;
    let stage = create_stage(stage_root)?;
    let payload = stage.path().join(match kind {
        Payload::Prefix => "update.tar.gz",
        Payload::AppImage => "update.AppImage",
        Payload::Any => "update.exe",
    });
    write_new(&payload, &bytes)?;
    drop(bytes); // Do not retain the download while waiting for the app's quit lifecycle.
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = if cfg!(windows) {
        "quickgui-updater-helper.exe"
    } else {
        "quickgui-updater-helper"
    };
    let source = executable
        .parent()
        .ok_or("invalid executable path")?
        .join(name);
    let helper = stage.path().join(name);
    fs::copy(source, &helper).map_err(|e| format!("could not stage updater helper: {e}"))?;
    let plan = Plan {
        parent: std::process::id(),
        target,
        kind,
        payload,
        item: item.clone(),
        public_key: key.into(),
    };
    let plan_path = stage.path().join("plan.json");
    write_new(
        &plan_path,
        &serde_json::to_vec(&plan).map_err(|e| e.to_string())?,
    )?;
    let log_path = stage.path().join("error.log");
    let log = fs::File::create(&log_path).map_err(|e| e.to_string())?;
    let mut command = Command::new(helper);
    command
        .arg("--install")
        .arg(&plan_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(log);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let output = child
        .stdout
        .take()
        .ok_or("updater helper has no readiness pipe")?;
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    // Exactly one bounded line. Kill a stuck helper so this reader is also released.
    std::thread::spawn(move || {
        let mut line = String::new();
        let result = BufReader::new(output)
            .take(128)
            .read_line(&mut line)
            .map(|_| line);
        let _ = tx.send(result);
    });
    let start = Instant::now();
    let prepared = loop {
        if cancelled() || start.elapsed() > READY_TIMEOUT {
            break Err("update handoff cancelled or timed out".into());
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(line)) if line.trim() == "READY" => break Ok(()),
            Ok(_) => {
                break Err(read_error(
                    &log_path,
                    "updater helper rejected the installation",
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break Err(read_error(
                    &log_path,
                    "updater helper exited before readiness",
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    if let Err(error) = prepared {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    // Ownership passes to the helper before the UI receives QuitRequired. If the app exits, the
    // helper cleans this directory; if quit is cancelled it exits after the bounded wait.
    let stage = stage.keep();
    ready();
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                let error = read_error(&log_path, "updater helper failed");
                let _ = fs::remove_dir_all(&stage);
                return if status.success() { Ok(()) } else { Err(error) };
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// Entry point for the bundled helper. It only accepts a private stage next to its own copied
/// executable, with a signed payload. No path or executable argument comes from the appcast.
pub fn run_helper() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--install" {
        return Err("expected --install <plan.json>".into());
    }
    let path = fs::canonicalize(&args[1]).map_err(|e| e.to_string())?;
    let stage = path.parent().ok_or("invalid update stage")?;
    let helper = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|e| e.to_string())?;
    if helper.parent() != Some(stage) || path.file_name() != Some(std::ffi::OsStr::new("plan.json"))
    {
        return Err("plan must be beside the staged helper".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|e| e.to_string())?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("update plan is too large".into());
    }
    let plan: Plan = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if plan.parent == 0 || plan.parent == std::process::id() {
        return Err("invalid update parent".into());
    }
    #[cfg(unix)]
    if plan.parent != unsafe { libc::getppid() } as u32 {
        return Err("update parent does not match the process that started the helper".into());
    }
    let payload = fs::canonicalize(&plan.payload).map_err(|e| e.to_string())?;
    if payload.parent() != Some(stage)
        || fs::canonicalize(&plan.target).map_err(|e| e.to_string())? != plan.target
    {
        return Err("invalid updater paths".into());
    }
    let bytes = read_payload(&payload)?;
    feed::verify(&bytes, &plan.item, &plan.public_key)?;
    validate_format(&bytes, plan.kind, &plan.target)?;
    drop(bytes);
    // Hold a handle to this exact parent on Windows, so PID reuse can never delay/advance install.
    let parent = Parent::open(plan.parent)?;
    fs::write(stage.join("owner"), std::process::id().to_string()).map_err(|e| e.to_string())?;
    println!("READY");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let result = (|| {
        parent.wait()?;
        // Validate again immediately before installation, including changes during the quit wait.
        let bytes = read_payload(&payload)?;
        feed::verify(&bytes, &plan.item, &plan.public_key)?;
        apply(&plan, bytes, stage)
    })();
    // Windows cannot unlink the running helper or its log, but release the large payload
    // on success and failure alike. The parent also removes the stage if it is still running.
    let _ = fs::remove_file(&plan.payload);
    let _ = fs::remove_file(&path);
    if result.is_ok() {
        let _ = fs::remove_dir_all(stage);
    }
    result
}

// Per-application stages are limited even across crashes. Never evict a live helper/parent.
// Successful Windows helpers leave their own locked executable for the next launch to remove.
fn create_stage(root: &Path) -> Result<tempfile::TempDir> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut retained = 0;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_name().to_string_lossy().starts_with("stage-")
            || !entry.file_type().map_err(|e| e.to_string())?.is_dir()
        {
            continue;
        }
        let owner = fs::File::open(entry.path().join("owner"))
            .ok()
            .and_then(|f| {
                let mut value = String::new();
                f.take(16).read_to_string(&mut value).ok()?;
                value.parse::<u32>().ok().filter(|id| *id > 0)
            });
        let abandoned = owner.map(|id| !process_alive(id)).unwrap_or_else(|| {
            entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > Duration::from_secs(86400))
        });
        if abandoned && fs::remove_dir_all(entry.path()).is_ok() {
            continue;
        }
        retained += 1;
        if retained >= 4 {
            return Err(format!(
                "update staging limit reached; close other app instances or remove stale stages in {}",
                root.display()
            ));
        }
    }
    let stage = tempfile::Builder::new()
        .prefix("stage-")
        .tempdir_in(root)
        .map_err(|e| e.to_string())?;
    write_new(
        &stage.path().join("owner"),
        std::process::id().to_string().as_bytes(),
    )?;
    Ok(stage)
}

fn process_alive(id: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError},
            System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject},
        };
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, id) };
        if handle.is_null() {
            return unsafe { GetLastError() } == ERROR_ACCESS_DENIED;
        }
        let active = unsafe { WaitForSingleObject(handle, 0) } != 0;
        unsafe { CloseHandle(handle) };
        active
    }
    #[cfg(not(windows))]
    {
        unsafe {
            libc::kill(id as i32, 0) == 0
                || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
}

fn read_payload(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > feed::MAX_DOWNLOAD_BYTES {
        return Err("update exceeds its size limit".into());
    }
    let mut bytes = Vec::new();
    file.take(feed::MAX_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > feed::MAX_DOWNLOAD_BYTES {
        return Err("update exceeds its size limit".into());
    }
    Ok(bytes)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
fn read_error(path: &Path, fallback: &str) -> String {
    let mut bytes = Vec::new();
    if let Ok(f) = fs::File::open(path) {
        let _ = f.take(16 * 1024).read_to_end(&mut bytes);
    }
    if bytes.is_empty() {
        fallback.into()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    }
}
fn validate_format(bytes: &[u8], kind: Payload, target: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    if kind == Payload::Prefix {
        return super::prefix::validate(bytes, target);
    }
    let _ = (kind, target);
    #[cfg(windows)]
    if !bytes.starts_with(b"MZ") {
        return Err("Windows updates must be signed QuickGUI NSIS executables".into());
    }
    #[cfg(not(windows))]
    if !bytes.starts_with(b"\x7fELF") || bytes.get(8..11) != Some(b"AI\x02") {
        return Err("Linux updates must be signed type-2 AppImages".into());
    }
    Ok(())
}

#[cfg(windows)]
struct Parent(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Parent {
    fn open(id: u32) -> Result<Self> {
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, id) };
        if handle.is_null() {
            Err("could not open update parent".into())
        } else {
            Ok(Self(handle))
        }
    }
    fn wait(&self) -> Result<()> {
        use windows_sys::Win32::System::Threading::WaitForSingleObject;
        if unsafe { WaitForSingleObject(self.0, QUIT_TIMEOUT.as_millis() as u32) } == 0 {
            Ok(())
        } else {
            Err("application did not quit; update cancelled".into())
        }
    }
}
#[cfg(windows)]
impl Drop for Parent {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}
#[cfg(not(windows))]
struct Parent {
    id: u32,
    #[cfg(target_os = "linux")]
    started: Option<String>,
}
#[cfg(not(windows))]
impl Parent {
    fn open(id: u32) -> Result<Self> {
        if unsafe { libc::kill(id as i32, 0) } != 0 {
            return Err("update parent is not running".into());
        }
        Ok(Self {
            id,
            #[cfg(target_os = "linux")]
            started: process_identity(id).map(|(_, started)| started),
        })
    }
    fn wait(&self) -> Result<()> {
        let start = Instant::now();
        loop {
            if unsafe { libc::kill(self.id as i32, 0) } != 0 {
                return Ok(());
            }
            #[cfg(target_os = "linux")]
            if process_identity(self.id).is_some_and(|(state, started)| {
                state == "Z"
                    || self
                        .started
                        .as_ref()
                        .is_some_and(|original| *original != started)
            }) {
                return Ok(());
            }
            if start.elapsed() > QUIT_TIMEOUT {
                return Err("application did not quit; update cancelled".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

fn apply(plan: &Plan, bytes: Vec<u8>, stage: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // Rewrite the verified bytes after waiting, rather than executing a file which could
        // have changed since verification. NSIS /D must be the final argument, without quotes.
        fs::write(&plan.payload, &bytes).map_err(|e| e.to_string())?;
        drop(bytes);
        run_windows_installer(
            &plan.payload,
            plan.target
                .parent()
                .ok_or("invalid installation directory")?,
        )?;
        relaunch(&plan.target, stage)
    }
    #[cfg(not(windows))]
    {
        #[cfg(target_os = "linux")]
        if plan.kind == Payload::Prefix {
            return super::prefix::apply(&plan.target, bytes, stage);
        }
        let parent = plan.target.parent().ok_or("invalid AppImage location")?;
        let backup = tempfile::Builder::new()
            .prefix(".quickgui-backup-")
            .tempdir_in(parent)
            .map_err(|e| e.to_string())?;
        let previous = backup.path().join("previous.AppImage");
        let replacement = backup.path().join("new.AppImage");
        let result = (|| {
            write_new(&replacement, &bytes)?;
            drop(bytes);
            fs::set_permissions(
                &replacement,
                fs::metadata(&plan.target)
                    .map_err(|e| e.to_string())?
                    .permissions(),
            )
            .map_err(|e| e.to_string())?;
            replace_with_backup(&plan.target, &replacement, &previous)?;
            if let Err(error) = relaunch(&plan.target, stage) {
                fs::rename(&previous, &plan.target).map_err(|e| {
                    format!(
                        "{error}; rollback failed: {e}; old AppImage: {}",
                        previous.display()
                    )
                })?;
                let _ = clean_command(&plan.target).spawn();
                return Err(format!("{error}; previous AppImage restored"));
            }
            Ok(())
        })();
        if result.is_err() && previous.exists() {
            let _ = backup.keep(); // Preserve the recovery copy only if restoring it failed.
        }
        result
    }
}

pub(crate) fn replace_with_backup(
    target: &Path,
    replacement: &Path,
    previous: &Path,
) -> Result<()> {
    fs::rename(target, previous).map_err(|e| e.to_string())?;
    if let Err(error) = fs::rename(replacement, target) {
        fs::rename(previous, target).map_err(|e| {
            format!(
                "replacement failed: {error}; rollback failed: {e}; backup: {}",
                previous.display()
            )
        })?;
        return Err(format!(
            "replacement failed; previous application restored: {error}"
        ));
    }
    if let Some(parent) = target.parent()
        && let Ok(directory) = fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}
pub(crate) fn clean_command(target: &Path) -> Command {
    let mut command = Command::new(target);
    for key in [
        "APPIMAGE",
        "APPDIR",
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "QUICKGUI_LIBRARY",
        "QUICKGUI_EXTENSION_DIR",
        READY_ENV,
    ] {
        command.env_remove(key);
    }
    command
        .current_dir(target.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
pub(crate) fn relaunch(target: &Path, stage: &Path) -> Result<()> {
    let ready = stage.join("application-ready");
    let _ = fs::remove_file(&ready);
    let mut child = clean_command(target)
        .env(READY_ENV, &ready)
        .spawn()
        .map_err(|e| format!("could not launch updated app: {e}"))?;
    let start = Instant::now();
    let mut acknowledged = None;
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            return Err("updated application exited before completing startup".into());
        }
        if ready.exists() && acknowledged.is_none() {
            acknowledged = Some(Instant::now());
        }
        if acknowledged.is_some_and(|time: Instant| time.elapsed() >= Duration::from_secs(3)) {
            return Ok(());
        }
        if start.elapsed() > READY_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err("updated application did not acknowledge startup".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(target_os = "linux")]
fn process_identity(id: u32) -> Option<(String, String)> {
    let mut bytes = String::new();
    fs::File::open(format!("/proc/{id}/stat"))
        .ok()?
        .take(4096)
        .read_to_string(&mut bytes)
        .ok()?;
    let fields: Vec<_> = bytes.rsplit_once(") ")?.1.split_whitespace().collect();
    Some((fields.first()?.to_string(), fields.get(19)?.to_string()))
}

#[cfg(windows)]
fn run_windows_installer(installer: &Path, directory: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::{
            Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
            Threading::{GetExitCodeProcess, WaitForSingleObject},
        },
        UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
    };
    let wide = |value: &std::ffi::OsStr| value.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let file = wide(installer.as_os_str());
    // /D must be the final NSIS parameter and is intentionally not quoted.
    let mut parameters: Vec<u16> = "/S /D=".encode_utf16().collect();
    parameters.extend(installer_directory(directory));
    parameters.push(0);
    let verb = wide(std::ffi::OsStr::new("open"));
    let initialized =
        unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) } >= 0;
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpVerb = verb.as_ptr();
    let accepted = unsafe { ShellExecuteExW(&mut info) } != 0;
    if initialized {
        unsafe { CoUninitialize() };
    }
    if !accepted || info.hProcess.is_null() {
        return Err(format!(
            "could not launch update installer: {}",
            std::io::Error::last_os_error()
        ));
    }
    let waited = unsafe { WaitForSingleObject(info.hProcess, 15 * 60 * 1000) };
    let mut exit = 0;
    let queried = unsafe { GetExitCodeProcess(info.hProcess, &mut exit) };
    unsafe { CloseHandle(info.hProcess) };
    if waited != 0 {
        return Err("update installer did not finish within 15 minutes".into());
    }
    if queried == 0 || exit != 0 {
        return Err(format!("update installer failed with status {exit}"));
    }
    Ok(())
}

#[cfg(windows)]
fn installer_directory(directory: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let path: Vec<u16> = directory.as_os_str().encode_wide().collect();
    // canonicalize() returns verbatim paths; NSIS requires the normal DOS/UNC spelling.
    let unc: Vec<u16> = "\\\\?\\UNC\\".encode_utf16().collect();
    let verbatim: Vec<u16> = "\\\\?\\".encode_utf16().collect();
    if let Some(rest) = path.strip_prefix(unc.as_slice()) {
        "\\\\".encode_utf16().chain(rest.iter().copied()).collect()
    } else {
        path.strip_prefix(verbatim.as_slice())
            .unwrap_or(&path)
            .to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn installer_accepts_canonical_disk_and_unc_directories() {
        assert_eq!(
            String::from_utf16(&installer_directory(Path::new(
                r"\\?\C:\Program Files\Test"
            )))
            .unwrap(),
            r"C:\Program Files\Test"
        );
        assert_eq!(
            String::from_utf16(&installer_directory(Path::new(
                r"\\?\UNC\server\share\Test"
            )))
            .unwrap(),
            r"\\server\share\Test"
        );
    }
    #[test]
    fn stages_have_a_fixed_limit_and_preserve_live_owners() {
        let dir = tempfile::tempdir().unwrap();
        let stages: Vec<_> = (0..4).map(|_| create_stage(dir.path()).unwrap()).collect();
        assert!(
            create_stage(dir.path())
                .unwrap_err()
                .contains("staging limit")
        );
        assert!(stages.iter().all(|stage| stage.path().exists()));
        drop(stages);
        assert!(create_stage(dir.path()).is_ok());
    }
    #[test]
    fn failed_replacement_restores_original() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("app");
        let backup = dir.path().join("old");
        fs::write(&target, b"original").unwrap();
        assert!(replace_with_backup(&target, &dir.path().join("missing"), &backup).is_err());
        assert_eq!(fs::read(target).unwrap(), b"original");
        assert!(!backup.exists());
    }
    #[test]
    fn replacement_retains_exact_backup() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("app");
        let new = dir.path().join("new");
        let old = dir.path().join("old");
        fs::write(&target, b"original").unwrap();
        fs::write(&new, b"updated").unwrap();
        replace_with_backup(&target, &new, &old).unwrap();
        assert_eq!(fs::read(target).unwrap(), b"updated");
        assert_eq!(fs::read(old).unwrap(), b"original");
    }
}
