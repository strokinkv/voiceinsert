#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderIndicatorColor {
    Recording,
    Transcribing,
}

#[cfg(windows)]
mod platform {
    use super::BorderIndicatorColor;
    use crate::overlay::primary_screen_area;
    use windows::Win32::Foundation::{COLORREF, ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND};
    use windows::Win32::Graphics::Gdi::{CreateSolidBrush, HBRUSH};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, LWA_ALPHA, RegisterClassW, SW_SHOWNA,
        SetLayeredWindowAttributes, ShowWindow, WINDOW_EX_STYLE, WM_CLOSE, WNDCLASSW,
        WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
        WS_POPUP,
    };
    use windows::core::PCWSTR;

    const BORDER_WIDTH: i32 = 12;
    const ALPHA: u8 = 210;

    pub struct BorderIndicator {
        windows: Vec<HWND>,
        visible_color: Option<BorderIndicatorColor>,
    }

    impl Default for BorderIndicator {
        fn default() -> Self {
            Self::new()
        }
    }

    impl BorderIndicator {
        pub fn new() -> Self {
            Self {
                windows: Vec::new(),
                visible_color: None,
            }
        }

        pub fn show(&mut self, color: BorderIndicatorColor) -> anyhow::Result<()> {
            if self.visible_color == Some(color) {
                return Ok(());
            }

            self.hide();
            let class_name = register_window_class(color)?;
            let area = primary_screen_area();
            let sides = [
                (area.left, area.top, area.width, BORDER_WIDTH),
                (
                    area.left,
                    area.top + area.height - BORDER_WIDTH,
                    area.width,
                    BORDER_WIDTH,
                ),
                (area.left, area.top, BORDER_WIDTH, area.height),
                (
                    area.left + area.width - BORDER_WIDTH,
                    area.top,
                    BORDER_WIDTH,
                    area.height,
                ),
            ];

            for (x, y, width, height) in sides {
                let hwnd = create_border_window(&class_name, x, y, width, height)?;
                self.windows.push(hwnd);
            }
            self.visible_color = Some(color);
            Ok(())
        }

        pub fn hide(&mut self) {
            for hwnd in self.windows.drain(..) {
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
            self.visible_color = None;
        }
    }

    impl Drop for BorderIndicator {
        fn drop(&mut self) {
            self.hide();
        }
    }

    fn register_window_class(color: BorderIndicatorColor) -> anyhow::Result<Vec<u16>> {
        let class_name = match color {
            BorderIndicatorColor::Recording => "VoiceInsertRecordingBorder",
            BorderIndicatorColor::Transcribing => "VoiceInsertTranscribingBorder",
        };
        let class_name = wide(class_name);
        let instance = unsafe { GetModuleHandleW(None)? };
        let brush = unsafe { CreateSolidBrush(color_ref(color)) };
        let window_class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hbrBackground: HBRUSH(brush.0),
            lpfnWndProc: Some(border_wnd_proc),
            ..Default::default()
        };

        let registered = unsafe { RegisterClassW(&window_class) };
        if registered == 0 && unsafe { GetLastError() } != ERROR_CLASS_ALREADY_EXISTS {
            anyhow::bail!("failed to register border indicator window class");
        }

        Ok(class_name)
    }

    fn create_border_window(
        class_name: &[u16],
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> anyhow::Result<HWND> {
        let styles: WINDOW_EX_STYLE =
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE;
        let hwnd = unsafe {
            CreateWindowExW(
                styles,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(wide("VoiceInsertBorder").as_ptr()),
                WS_POPUP,
                x,
                y,
                width,
                height,
                None,
                None,
                Some(GetModuleHandleW(None)?.into()),
                None,
            )?
        };
        unsafe {
            SetLayeredWindowAttributes(hwnd, COLORREF(0), ALPHA, LWA_ALPHA)?;
            let _ = ShowWindow(hwnd, SW_SHOWNA);
        }
        Ok(hwnd)
    }

    fn color_ref(color: BorderIndicatorColor) -> COLORREF {
        match color {
            BorderIndicatorColor::Recording => COLORREF(0x46c978),
            BorderIndicatorColor::Transcribing => COLORREF(0x2d8cff),
        }
    }

    unsafe extern "system" fn border_wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT {
        if msg == WM_CLOSE {
            return windows::Win32::Foundation::LRESULT(0);
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::BorderIndicatorColor;

    pub struct BorderIndicator;

    impl BorderIndicator {
        pub fn new() -> Self {
            Self
        }

        pub fn show(&mut self, _color: BorderIndicatorColor) -> anyhow::Result<()> {
            Ok(())
        }

        pub fn hide(&mut self) {}
    }
}

pub use platform::BorderIndicator;
