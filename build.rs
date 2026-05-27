fn main() {
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=ui/overlay.slint");
    slint_build::compile("ui/settings.slint").expect("failed to compile settings UI");
    slint_build::compile("ui/overlay.slint").expect("failed to compile overlay UI");
}
