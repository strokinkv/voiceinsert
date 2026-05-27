fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=ui/overlay.slint");
    slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");
}
