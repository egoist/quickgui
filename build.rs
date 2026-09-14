use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(quickgui_terminal_extension)");
    println!("cargo:rustc-check-cfg=cfg(quickgui_component_extension)");
    println!("cargo:rerun-if-changed=src/macos/swift_ui.swift");

    if env::var_os("CARGO_FEATURE_SWIFT_UI").is_none() {
        return;
    }

    let target = env::var("TARGET").expect("Cargo always supplies TARGET to build scripts");
    if !target.ends_with("apple-darwin") {
        return;
    }

    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").expect("Cargo always supplies OUT_DIR to build scripts"),
    );
    let source = Path::new("src/macos/swift_ui.swift");
    let library = out_dir.join("libquickgui_swift_ui.a");
    let sdk = command_text("xcrun", &["--sdk", "macosx", "--show-sdk-path"]);
    let swiftc = command_text("xcrun", &["--find", "swiftc"]);
    let deployment = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".to_owned());
    let architecture = if target.starts_with("aarch64-") {
        "arm64"
    } else if target.starts_with("x86_64-") {
        "x86_64"
    } else {
        panic!("the SwiftUI bridge does not support target {target}");
    };
    let swift_target = format!("{architecture}-apple-macosx{deployment}");

    let output = Command::new(swiftc.trim())
        .args([
            "-emit-library",
            "-static",
            "-parse-as-library",
            "-O",
            "-swift-version",
            "5",
            "-Xfrontend",
            "-disable-implicit-concurrency-module-import",
            "-Xfrontend",
            "-disable-autolinking-runtime-compatibility-concurrency",
            "-Xfrontend",
            "-disable-autolink-library",
            "-Xfrontend",
            "swift_Concurrency",
            "-module-name",
            "QuickGUISwiftUIBridge",
            "-target",
            &swift_target,
            "-sdk",
            sdk.trim(),
        ])
        .arg(source)
        .arg("-o")
        .arg(&library)
        .output()
        .unwrap_or_else(|error| panic!("could not launch swiftc for the SwiftUI bridge: {error}"));
    require_success("swiftc", &output);

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=quickgui_swift_ui");
    println!(
        "cargo:rustc-link-search=native={}/usr/lib/swift",
        sdk.trim()
    );
    // SwiftUI's Xcode 26 control bindings reference MainActor metadata even though this bridge
    // exposes a synchronous C ABI. Swift autolinking is disabled above so Rust remains in charge
    // of the final link; name the one Swift runtime dylib those bindings require explicitly.
    println!("cargo:rustc-link-lib=dylib=swift_Concurrency");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    println!("cargo:rustc-link-lib=framework=SwiftUI");
    println!("cargo:rustc-link-lib=framework=AppKit");
}

fn command_text(command: &str, arguments: &[&str]) -> String {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("could not launch {command}: {error}"));
    require_success(command, &output);
    String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("{command} returned non-UTF-8 output: {error}"))
}

fn require_success(command: &str, output: &Output) {
    if output.status.success() {
        return;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    panic!("{command} failed\n{stdout}{stderr}");
}
