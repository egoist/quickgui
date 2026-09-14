#[cfg(not(target_os = "macos"))]
fn main() {
    if let Err(error) = quickgui_updater::handoff::run_helper() {
        eprintln!("QuickGUI updater: {error}");
        std::process::exit(1);
    }
}
#[cfg(target_os = "macos")]
fn main() {}
