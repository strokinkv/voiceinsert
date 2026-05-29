fn main() {
    println!("cargo:rerun-if-changed=ui/app.slint");
    println!("cargo:rerun-if-changed=ui/settings.slint");
    println!("cargo:rerun-if-changed=ui/overlay.slint");
    println!("cargo:rerun-if-changed=assets/VoiceInsert.ico");
    slint_build::compile("ui/app.slint").expect("failed to compile Slint UI");

    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/VoiceInsert.ico");
        resource
            .compile()
            .expect("failed to embed Windows executable icon");
    }
}
