//! Windows MSVC link helpers for the terminal cdylib.
//!
//! `libghostty-vt-sys` 0.2.1 emits `static=ghostty-vt`. On MSVC rustc resolves that
//! to `ghostty-vt.lib`, which is Zig's DLL import library, not `ghostty-vt-static.lib`.
//! Linking the import library into this cdylib pulls `msvcrt.lib` startup objects
//! without the rest of the CRT, so `link.exe` reports unresolved `memcpy` and
//! `__CxxFrameHandler3`. Staging the static archive keeps rustc on its usual
//! `/defaultlib:msvcrt` line. Do not also link `libucrt` / `libvcruntime`; those
//! collide with `ucrt.lib` (`LNK2005` on `_initialize_narrow_environment`).
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const STATIC_ARCHIVE: &str = "ghostty-vt-static.lib";
pub const IMPORT_LIBRARY: &str = "ghostty-vt.lib";

pub fn needs_msvc_ghostty_link(target: &str) -> bool {
    target.contains("windows-msvc")
}

pub fn lib_dir_from_include(include: &str) -> Option<PathBuf> {
    env::split_paths(include).find_map(|include_dir| {
        let lib_dir = include_dir.parent()?.join("lib");
        lib_dir.join(STATIC_ARCHIVE).is_file().then_some(lib_dir)
    })
}

pub fn lib_dir_from_cargo_build(build_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(build_dir).ok()?;
    entries.flatten().find_map(|entry| {
        let name = entry.file_name();
        if !name.to_string_lossy().starts_with("libghostty-vt-sys-") {
            return None;
        }
        let lib_dir = entry.path().join("out").join("ghostty-install").join("lib");
        lib_dir.join(STATIC_ARCHIVE).is_file().then_some(lib_dir)
    })
}

pub fn stage_static_ghostty_archive(lib_dir: &Path) -> io::Result<bool> {
    let static_lib = lib_dir.join(STATIC_ARCHIVE);
    if !static_lib.is_file() {
        return Ok(false);
    }
    fs::copy(&static_lib, lib_dir.join(IMPORT_LIBRARY))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = env::temp_dir().join(format!("quickgui-ghostty-{name}-{stamp}"));
        fs::create_dir_all(root.join("lib")).expect("scratch lib dir");
        root
    }

    #[test]
    fn msvc_targets_need_the_ghostty_link_fix() {
        assert!(needs_msvc_ghostty_link("x86_64-pc-windows-msvc"));
        assert!(needs_msvc_ghostty_link("aarch64-pc-windows-msvc"));
        assert!(!needs_msvc_ghostty_link("x86_64-pc-windows-gnu"));
        assert!(!needs_msvc_ghostty_link("x86_64-unknown-linux-gnu"));
        assert!(!needs_msvc_ghostty_link("aarch64-apple-darwin"));
    }

    #[test]
    fn include_metadata_points_at_the_static_archive() {
        let root = scratch("include");
        let include = root.join("include");
        let lib = root.join("lib");
        fs::create_dir_all(&include).unwrap();
        fs::write(lib.join(STATIC_ARCHIVE), b"static").unwrap();
        assert_eq!(lib_dir_from_include(&include.to_string_lossy()), Some(lib));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cargo_build_dir_finds_the_sys_crate_install() {
        let root = scratch("build");
        let lib = root
            .join("libghostty-vt-sys-deadbeef")
            .join("out")
            .join("ghostty-install")
            .join("lib");
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join(STATIC_ARCHIVE), b"static").unwrap();
        assert_eq!(lib_dir_from_cargo_build(&root), Some(lib));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stages_the_static_archive_over_the_import_library() {
        let root = scratch("stage");
        let lib = root.join("lib");
        fs::write(lib.join(STATIC_ARCHIVE), b"static-archive").unwrap();
        fs::write(lib.join(IMPORT_LIBRARY), b"import-library").unwrap();
        assert!(stage_static_ghostty_archive(&lib).unwrap());
        assert_eq!(
            fs::read(lib.join(IMPORT_LIBRARY)).unwrap(),
            b"static-archive"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staging_is_a_no_op_without_the_static_archive() {
        let root = scratch("missing");
        let lib = root.join("lib");
        assert!(!stage_static_ghostty_archive(&lib).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
