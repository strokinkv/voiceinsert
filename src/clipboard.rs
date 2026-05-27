use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardInserter {
    pub restore_clipboard: bool,
    pub delay_before_paste_ms: u64,
    pub delay_before_restore_ms: u64,
}

impl ClipboardInserter {
    pub async fn insert_text(
        &self,
        text: &str,
        target_window: Option<isize>,
    ) -> anyhow::Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        let previous_text = if self.restore_clipboard {
            clipboard.get_text().ok()
        } else {
            None
        };

        clipboard.set_text(text.to_string())?;
        sleep_ms(self.delay_before_paste_ms).await;

        if let Some(window) = target_window {
            focus_window(window);
            sleep_ms(40).await;
        }

        send_ctrl_v();

        if self.restore_clipboard
            && let Some(previous_text) = previous_text
        {
            sleep_ms(self.delay_before_restore_ms).await;
            let _ = clipboard.set_text(previous_text);
        }

        Ok(())
    }
}

async fn sleep_ms(milliseconds: u64) {
    if milliseconds > 0 {
        tokio::time::sleep(Duration::from_millis(milliseconds)).await;
    }
}

#[cfg(windows)]
fn focus_window(target_window: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

    if target_window != 0 {
        unsafe {
            let _ = SetForegroundWindow(HWND(target_window as *mut _));
        }
    }
}

#[cfg(not(windows))]
fn focus_window(_target_window: isize) {}

#[cfg(windows)]
fn send_ctrl_v() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY,
        VK_CONTROL, VK_V,
    };

    let inputs = [
        keyboard_input(VK_CONTROL, Default::default()),
        keyboard_input(VK_V, Default::default()),
        keyboard_input(VK_V, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];

    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    fn keyboard_input(key: VIRTUAL_KEY, flags: KEYEVENTF_KEYUP) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}

#[cfg(not(windows))]
fn send_ctrl_v() {}
