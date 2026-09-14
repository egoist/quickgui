fn main() {
    println!("cargo:rustc-check-cfg=cfg(quickgui_component_extension)");
    println!("cargo:rustc-cfg=quickgui_component_extension");
}
