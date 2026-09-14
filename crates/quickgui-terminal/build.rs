#[path = "windows_msvc_link.rs"]
mod windows_msvc_link;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(quickgui_terminal_extension)");
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"terminal\"))");
    println!("cargo:rustc-cfg=quickgui_terminal_extension");
    configure_windows_msvc_ghostty();
}

fn configure_windows_msvc_ghostty() {
    let target = env::var("TARGET").unwrap_or_default();
    if !windows_msvc_link::needs_msvc_ghostty_link(&target) {
        return;
    }

    let lib_dir = env::var("DEP_GHOSTTY_VT_INCLUDE")
        .ok()
        .and_then(|include| windows_msvc_link::lib_dir_from_include(&include))
        .or_else(|| {
            let out_dir = PathBuf::from(env::var_os("OUT_DIR")?);
            windows_msvc_link::lib_dir_from_cargo_build(out_dir.parent()?.parent()?)
        })
        .unwrap_or_else(|| {
            panic!(
                "libghostty-vt-sys did not produce {} for {target}; \
                 the Windows terminal cdylib must link the static Ghostty archive",
                windows_msvc_link::STATIC_ARCHIVE
            )
        });

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!(
        "cargo:rerun-if-changed={}",
        lib_dir.join(windows_msvc_link::STATIC_ARCHIVE).display()
    );
    if !windows_msvc_link::stage_static_ghostty_archive(&lib_dir).unwrap_or(false) {
        panic!(
            "failed to stage {} as {} in {}",
            windows_msvc_link::STATIC_ARCHIVE,
            windows_msvc_link::IMPORT_LIBRARY,
            lib_dir.display()
        );
    }
}
