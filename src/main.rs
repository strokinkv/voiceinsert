#![windows_subsystem = "windows"]

fn main() -> anyhow::Result<()> {
    set_stable_dpi_awareness();
    voiceinsert::app::run()
}

#[cfg(windows)]
fn set_stable_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, SetProcessDpiAwarenessContext,
    };

    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE) };
}

#[cfg(not(windows))]
fn set_stable_dpi_awareness() {}
