//! The per-user install `install.sh` unpacks: one `bin/` + `share/` prefix the user owns, carrying
//! a marker. Only such a prefix replaces itself; `/usr`, `.deb` payloads, and unpacked AppDirs
//! have no marker and stay with their package manager. Updates swap the complete directory, so a
//! file dropped from a later layout never survives, and the previous prefix is kept until the new
//! application acknowledges startup.
use super::handoff::{clean_command, relaunch, replace_with_backup};
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Component, Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;

const MARKER: &str = "share/quickgui/install.json";
const MAX_MARKER_BYTES: u64 = 4096;
const MAX_ENTRIES: usize = 65_536;
const MAX_EXPANDED_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_DESKTOP_ENTRY_BYTES: u64 = 64 * 1024;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct Marker {
    schema: u32,
    identifier: String,
    executable: String,
}

fn parse_marker(bytes: &[u8]) -> Result<Marker> {
    let marker: Marker =
        serde_json::from_slice(bytes).map_err(|_| "invalid managed-install marker")?;
    let name = Path::new(&marker.executable);
    if marker.schema != 1
        || marker.identifier.is_empty()
        || name.components().count() != 1
        || !matches!(name.components().next(), Some(Component::Normal(_)))
    {
        return Err("unsupported managed-install marker".into());
    }
    Ok(marker)
}

fn read_marker(prefix: &Path) -> Result<Marker> {
    let mut bytes = Vec::new();
    fs::File::open(prefix.join(MARKER))
        .map_err(|_| "installation has no managed-install marker")?
        .take(MAX_MARKER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_MARKER_BYTES {
        return Err("managed-install marker is too large".into());
    }
    parse_marker(&bytes)
}

/// The prefix that holds `executable` (already canonical), when the current user may replace it.
pub(crate) fn locate(executable: &Path) -> Result<PathBuf> {
    let bin = executable.parent().ok_or("invalid executable path")?;
    let prefix = bin
        .parent()
        .filter(|_| bin.file_name() == Some("bin".as_ref()))
        .ok_or("the application is not inside a managed installation")?;
    let marker = read_marker(prefix)?;
    if executable.file_name() != Some(marker.executable.as_ref()) {
        return Err("managed-install marker names a different application".into());
    }
    let metadata = fs::metadata(prefix).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err("the installation must be owned by the current user".into());
    }
    let parent = prefix.parent().ok_or("invalid installation location")?;
    tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "the installation's parent directory must be writable")?;
    Ok(prefix.to_path_buf())
}

/// Visit every member below the archive's single top-level directory. Only regular files and
/// directories are accepted, so nothing unpacked can point outside its destination.
fn walk(
    bytes: &[u8],
    mut visit: impl FnMut(&Path, &mut tar::Entry<'_, flate2::read::GzDecoder<&[u8]>>) -> Result<()>,
) -> Result<()> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    let mut root: Option<PathBuf> = None;
    let mut expanded = 0u64;
    for (index, entry) in archive
        .entries()
        .map_err(|e| format!("invalid update archive: {e}"))?
        .enumerate()
    {
        let mut entry = entry.map_err(|e| format!("invalid update archive: {e}"))?;
        if index >= MAX_ENTRIES {
            return Err("update archive has too many entries".into());
        }
        expanded = expanded.saturating_add(entry.size());
        if expanded > MAX_EXPANDED_BYTES {
            return Err("update archive expands beyond its size limit".into());
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err("update archives may only hold files and directories".into());
        }
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let mut components = path.components();
        let Some(Component::Normal(first)) = components.next() else {
            return Err("update archive entry escapes its directory".into());
        };
        if root.get_or_insert_with(|| first.into()).as_os_str() != first {
            return Err("update archive must hold one top-level directory".into());
        }
        let rest = components.as_path();
        if rest
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("update archive entry escapes its directory".into());
        }
        if !rest.as_os_str().is_empty() {
            visit(rest, &mut entry)?;
        }
    }
    Ok(())
}

/// Read the archive without writing anything: its marker and the executable that marker names.
fn inspect(bytes: &[u8]) -> Result<Marker> {
    let mut marker = None;
    let mut executables = Vec::new();
    walk(bytes, |path, entry| {
        if path == Path::new(MARKER) {
            let mut bytes = Vec::new();
            entry
                .take(MAX_MARKER_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            marker = Some(parse_marker(&bytes)?);
        } else if path.parent() == Some(Path::new("bin"))
            && entry.header().entry_type().is_file()
            && entry.header().mode().is_ok_and(|mode| mode & 0o100 != 0)
        {
            executables.push(path.to_path_buf());
        }
        Ok(())
    })?;
    let marker = marker.ok_or("update archive has no managed-install marker")?;
    if !executables.contains(&Path::new("bin").join(&marker.executable)) {
        return Err("update archive is missing its executable".into());
    }
    Ok(marker)
}

/// Reject, before the application is asked to quit, an archive that `apply` would refuse.
pub(crate) fn validate(bytes: &[u8], prefix: &Path) -> Result<()> {
    if inspect(bytes)?.identifier != read_marker(prefix)?.identifier {
        return Err("update archive belongs to a different application".into());
    }
    Ok(())
}

fn unpack(bytes: &[u8], destination: &Path) -> Result<()> {
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    walk(bytes, |path, entry| {
        let target = destination.join(path);
        if entry.header().entry_type().is_dir() {
            return fs::create_dir_all(&target).map_err(|e| e.to_string());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let executable = entry.header().mode().is_ok_and(|mode| mode & 0o100 != 0);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        let size = entry.size();
        if std::io::copy(&mut entry.take(size), &mut file).map_err(|e| e.to_string())? != size {
            return Err("update archive is truncated".into());
        }
        // Never carry setuid, setgid, or group/world-writable bits out of an archive.
        file.set_permissions(fs::Permissions::from_mode(if executable {
            0o755
        } else {
            0o644
        }))
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
    })
}

pub(crate) fn apply(prefix: &Path, bytes: Vec<u8>, stage: &Path) -> Result<()> {
    let current = read_marker(prefix)?;
    let parent = prefix.parent().ok_or("invalid installation location")?;
    let backup = tempfile::Builder::new()
        .prefix(".quickgui-backup-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let previous = backup.path().join("previous");
    let replacement = backup.path().join("new");
    let result = (|| {
        unpack(&bytes, &replacement)?;
        drop(bytes);
        let next = read_marker(&replacement)?;
        if next.identifier != current.identifier {
            return Err("update archive belongs to a different application".into());
        }
        let metadata = fs::metadata(replacement.join("bin").join(&next.executable))
            .map_err(|_| "update archive is missing its executable")?;
        if !metadata.is_file() || metadata.mode() & 0o100 == 0 {
            return Err("update archive is missing its executable".into());
        }
        replace_with_backup(prefix, &replacement, &previous)?;
        if let Err(error) = relaunch(&prefix.join("bin").join(&next.executable), stage) {
            fs::rename(prefix, backup.path().join("failed"))
                .and_then(|_| fs::rename(&previous, prefix))
                .map_err(|e| {
                    format!(
                        "{error}; rollback failed: {e}; old installation: {}",
                        previous.display()
                    )
                })?;
            let _ = clean_command(&prefix.join("bin").join(&current.executable)).spawn();
            return Err(format!("{error}; previous installation restored"));
        }
        refresh_desktop_entry(prefix, &next);
        Ok(())
    })();
    if result.is_err() && previous.exists() {
        let _ = backup.keep(); // Preserve the recovery copy only if restoring it failed.
    }
    result
}

/// `install.sh` pinned the launcher entry to this prefix. Carry a later release's entry (new URL
/// schemes or document types) over to it; an entry that points elsewhere is someone else's.
fn refresh_desktop_entry(prefix: &Path, marker: &Marker) {
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
    {
        refresh_registrations(prefix, marker, &data_home);
    }
}

fn refresh_registrations(prefix: &Path, marker: &Marker, data_home: &Path) {
    let Some(prefix_text) = prefix
        .to_str()
        .filter(|text| !text.contains(['"', '`', '$', '\\', '\n']))
    else {
        return;
    };
    let name = format!("{}.desktop", marker.executable);
    let installed = data_home.join("applications").join(&name);
    let read = |path: &Path| {
        let mut text = String::new();
        fs::File::open(path)
            .ok()?
            .take(MAX_DESKTOP_ENTRY_BYTES)
            .read_to_string(&mut text)
            .ok()?;
        Some(text)
    };
    let executable = format!("\"{prefix_text}/bin/{}\"", marker.executable);
    if !read(&installed).is_some_and(|text| text.contains(&executable)) {
        return;
    }
    let Some(packaged) = read(&prefix.join("share/applications").join(&name)) else {
        return;
    };
    let icon = prefix
        .join("share/icons/hicolor/256x256/apps")
        .join(format!("{}.png", marker.executable));
    let text = pin_desktop_entry(
        &packaged,
        &marker.executable,
        &executable,
        icon.is_file().then_some(icon.as_path()),
    );
    write_registration(&installed, text.as_bytes());
    let _ = database_command("update-desktop-database", &data_home.join("applications")).status();

    // File types follow the release as well: a new package replaces the registered one, and a
    // release that dropped its document types withdraws it.
    let name = format!("{}.xml", marker.executable);
    let registered = data_home.join("mime/packages").join(&name);
    match read(&prefix.join("share/mime/packages").join(&name)) {
        Some(package) => write_registration(&registered, package.as_bytes()),
        None if registered.is_file() => {
            let _ = fs::remove_file(&registered);
        }
        None => return,
    }
    let _ = database_command("update-mime-database", &data_home.join("mime")).status();
}

/// Replace one registration file atomically. Failures leave the previous registration in place.
fn write_registration(path: &Path, contents: &[u8]) {
    let Some(directory) = path.parent() else {
        return;
    };
    if fs::create_dir_all(directory).is_ok()
        && let Ok(mut file) = tempfile::NamedTempFile::new_in(directory)
        && file.write_all(contents).is_ok()
        && file
            .as_file()
            .set_permissions(fs::Permissions::from_mode(0o644))
            .is_ok()
    {
        let _ = file.persist(path);
    }
}

/// The desktop's cache refresh tools are optional; a missing one only delays the change.
fn database_command(tool: &str, directory: &Path) -> std::process::Command {
    let mut command = std::process::Command::new(tool);
    command
        .arg(directory)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command
}

/// The same rewrite `install.sh` applies to the relocatable packaged entry.
fn pin_desktop_entry(packaged: &str, name: &str, executable: &str, icon: Option<&Path>) -> String {
    let bare_exec = format!("Exec={name}");
    let bare_icon = format!("Icon={name}");
    packaged
        .lines()
        .map(|line| match (line.strip_prefix(&bare_exec), icon) {
            (Some(arguments), _) => format!("Exec={executable}{arguments}\n"),
            (None, Some(icon)) if line == bare_icon => format!("Icon={}\n", icon.display()),
            _ => format!("{line}\n"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(root: &str, identifier: &str, members: &[(&str, &[u8], u32)]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let marker = format!(r#"{{"schema":1,"identifier":"{identifier}","executable":"app"}}"#);
        for (path, data, mode) in
            members
                .iter()
                .copied()
                .chain([(MARKER, marker.as_bytes(), 0o644)])
        {
            let mut header = tar::Header::new_ustar();
            header.set_size(data.len() as u64);
            header.set_mode(mode);
            header.set_cksum();
            builder
                .append_data(&mut header, format!("{root}/{path}"), data)
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn install(prefix: &Path, bytes: &[u8]) {
        unpack(bytes, prefix).unwrap();
    }

    #[test]
    fn unpacks_one_prefix_with_sanitized_modes() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = archive(
            "app-2.0.0",
            "com.example.app",
            &[
                ("bin/app", b"new", 0o4777),
                ("bin/assets/note.txt", b"n", 0o666),
            ],
        );
        assert_eq!(inspect(&bytes).unwrap().identifier, "com.example.app");
        let prefix = dir.path().join("prefix");
        install(&prefix, &bytes);
        let mode = |path: &str| fs::metadata(prefix.join(path)).unwrap().mode() & 0o7777;
        assert_eq!(mode("bin/app"), 0o755);
        assert_eq!(mode("bin/assets/note.txt"), 0o644);
        assert_eq!(read_marker(&prefix).unwrap().executable, "app");
        assert_eq!(locate(&prefix.join("bin/app")).unwrap(), prefix);
        assert!(locate(&prefix.join("bin/assets/note.txt")).is_err());
    }

    #[test]
    fn rejects_foreign_incomplete_and_escaping_archives() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix");
        install(
            &prefix,
            &archive(
                "app-1.0.0",
                "com.example.app",
                &[("bin/app", b"old", 0o755)],
            ),
        );
        let foreign = archive(
            "app-2.0.0",
            "com.example.other",
            &[("bin/app", b"x", 0o755)],
        );
        assert!(
            validate(&foreign, &prefix)
                .unwrap_err()
                .contains("different application")
        );
        let inert = archive("app-2.0.0", "com.example.app", &[("bin/app", b"x", 0o644)]);
        assert!(
            validate(&inert, &prefix)
                .unwrap_err()
                .contains("executable")
        );
        assert!(inspect(b"\x1f\x8bnot gzip").is_err());

        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        builder
            .append_link(&mut header, "app-2.0.0/bin/app", "/bin/sh")
            .unwrap();
        let link = builder.into_inner().unwrap().finish().unwrap();
        assert!(inspect(&link).unwrap_err().contains("only hold files"));

        let mut two_roots = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for path in ["a/bin/app", "b/bin/app"] {
            let mut header = tar::Header::new_ustar();
            header.set_size(1);
            header.set_mode(0o755);
            header.set_cksum();
            two_roots.append_data(&mut header, path, &b"x"[..]).unwrap();
        }
        let two_roots = two_roots.into_inner().unwrap().finish().unwrap();
        assert!(inspect(&two_roots).unwrap_err().contains("one top-level"));
    }

    #[test]
    fn swaps_the_prefix_and_restores_it_when_the_new_build_fails_to_start() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix");
        let stage = dir.path().join("stage");
        fs::create_dir(&stage).unwrap();
        let healthy: &[u8] = b"#!/bin/sh\n: >\"$QUICKGUI_UPDATE_READY_FILE\"\nsleep 5\n";
        install(
            &prefix,
            &archive(
                "app-1.0.0",
                "com.example.app",
                &[
                    ("bin/app", healthy, 0o755),
                    ("bin/dropped.txt", b"1", 0o644),
                ],
            ),
        );
        let broken = archive(
            "app-2.0.0",
            "com.example.app",
            &[("bin/app", b"#!/bin/sh\nexit 1\n", 0o755)],
        );
        let error = apply(&prefix, broken, &stage).unwrap_err();
        assert!(error.contains("previous installation restored"), "{error}");
        assert!(prefix.join("bin/dropped.txt").exists());

        let update = archive(
            "app-2.0.0",
            "com.example.app",
            &[("bin/app", healthy, 0o755), ("bin/added.txt", b"2", 0o644)],
        );
        apply(&prefix, update, &stage).unwrap();
        assert!(prefix.join("bin/added.txt").exists());
        assert!(!prefix.join("bin/dropped.txt").exists());
        // Neither outcome leaves a backup beside the installation.
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn an_update_refreshes_only_registrations_that_point_at_this_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let prefix = dir.path().join("prefix");
        let data_home = dir.path().join("data");
        let entry = "[Desktop Entry]\nExec=app %U\nIcon=app\nMimeType=x-scheme-handler/new;\n";
        install(
            &prefix,
            &archive(
                "app-2.0.0",
                "com.example.app",
                &[
                    ("bin/app", b"x", 0o755),
                    ("share/applications/app.desktop", entry.as_bytes(), 0o644),
                    ("share/mime/packages/app.xml", b"<new/>", 0o644),
                ],
            ),
        );
        let marker = read_marker(&prefix).unwrap();
        let registered = data_home.join("applications/app.desktop");
        let mime = data_home.join("mime/packages/app.xml");
        fs::create_dir_all(registered.parent().unwrap()).unwrap();

        // Another installation owns the launcher entry: nothing of it is touched.
        fs::write(&registered, "Exec=\"/opt/other/bin/app\"\n").unwrap();
        refresh_registrations(&prefix, &marker, &data_home);
        assert_eq!(
            fs::read_to_string(&registered).unwrap(),
            "Exec=\"/opt/other/bin/app\"\n"
        );
        assert!(!mime.exists());

        fs::write(
            &registered,
            format!("Exec=\"{}/bin/app\"\n", prefix.display()),
        )
        .unwrap();
        refresh_registrations(&prefix, &marker, &data_home);
        let text = fs::read_to_string(&registered).unwrap();
        assert!(text.contains(&format!("Exec=\"{}/bin/app\" %U\n", prefix.display())));
        assert!(text.contains("x-scheme-handler/new"));
        assert_eq!(fs::read_to_string(&mime).unwrap(), "<new/>");

        // A later release without document types withdraws the package.
        fs::remove_file(prefix.join("share/mime/packages/app.xml")).unwrap();
        refresh_registrations(&prefix, &marker, &data_home);
        assert!(!mime.exists());
    }

    #[test]
    fn pins_the_packaged_desktop_entry_to_the_prefix() {
        let packaged = "[Desktop Entry]\nExec=app %U\nIcon=app\nStartupWMClass=app\n";
        assert_eq!(
            pin_desktop_entry(
                packaged,
                "app",
                "\"/home/u/.local/app.app/bin/app\"",
                Some(Path::new("/home/u/icon.png"))
            ),
            "[Desktop Entry]\nExec=\"/home/u/.local/app.app/bin/app\" %U\nIcon=/home/u/icon.png\nStartupWMClass=app\n"
        );
        assert!(pin_desktop_entry(packaged, "app", "\"/x\"", None).contains("Icon=app\n"));
    }
}
