use std::time::Duration;

/// Inserts recognized text into the active target using clipboard paste semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardInserter {
    pub restore_clipboard: bool,
    pub delay_before_paste_ms: u64,
    pub delay_before_restore_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsertAction {
    SleepBeforePaste(u64),
    FocusWindow(isize),
    SleepAfterFocus(u64),
    Paste,
    SleepBeforeRestore(u64),
    RestoreClipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Control,
    V,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyAction {
    Press(Key),
    Release(Key),
}

impl ClipboardInserter {
    /// Copies `text`, focuses `target_window` when provided, sends Ctrl+V, and optionally restores the previous text clipboard.
    pub async fn insert_text(
        &self,
        text: &str,
        target_window: Option<isize>,
    ) -> anyhow::Result<()> {
        let mut clipboard = arboard::Clipboard::new()?;
        let previous_text = if self.restore_clipboard {
            match clipboard.get_text() {
                Ok(text) => Some(text),
                Err(error) => {
                    tracing::debug!(%error, "clipboard did not contain readable text");
                    None
                }
            }
        } else {
            None
        };

        clipboard.set_text(text.to_string())?;

        for action in plan_insert_actions(self, target_window, previous_text.is_some()) {
            match action {
                InsertAction::SleepBeforePaste(milliseconds)
                | InsertAction::SleepAfterFocus(milliseconds)
                | InsertAction::SleepBeforeRestore(milliseconds) => {
                    sleep_ms(milliseconds).await;
                }
                InsertAction::FocusWindow(window) => focus_window(window),
                InsertAction::Paste => send_ctrl_v(),
                InsertAction::RestoreClipboard => {
                    if let Some(previous_text) = previous_text.as_ref() {
                        let _ = clipboard.set_text(previous_text.to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

fn plan_insert_actions(
    inserter: &ClipboardInserter,
    target_window: Option<isize>,
    has_previous_text: bool,
) -> Vec<InsertAction> {
    let mut actions = vec![InsertAction::SleepBeforePaste(
        inserter.delay_before_paste_ms,
    )];

    if let Some(window) = target_window
        && window != 0
    {
        actions.push(InsertAction::FocusWindow(window));
        actions.push(InsertAction::SleepAfterFocus(40));
    }

    actions.push(InsertAction::Paste);

    if inserter.restore_clipboard && has_previous_text {
        actions.push(InsertAction::SleepBeforeRestore(
            inserter.delay_before_restore_ms,
        ));
        actions.push(InsertAction::RestoreClipboard);
    }

    actions
}

fn ctrl_v_key_sequence() -> [KeyAction; 4] {
    [
        KeyAction::Press(Key::Control),
        KeyAction::Press(Key::V),
        KeyAction::Release(Key::V),
        KeyAction::Release(Key::Control),
    ]
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

    let inputs = ctrl_v_key_sequence().map(keyboard_input);

    unsafe {
        let _ = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }

    fn keyboard_input(action: KeyAction) -> INPUT {
        let (key, flags) = match action {
            KeyAction::Press(key) => (virtual_key(key), Default::default()),
            KeyAction::Release(key) => (virtual_key(key), KEYEVENTF_KEYUP),
        };

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

    fn virtual_key(key: Key) -> VIRTUAL_KEY {
        match key {
            Key::Control => VK_CONTROL,
            Key::V => VK_V,
        }
    }
}

#[cfg(not(windows))]
fn send_ctrl_v() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_insert_actions_skip_focus_for_zero_window() {
        let inserter = ClipboardInserter {
            restore_clipboard: true,
            delay_before_paste_ms: 10,
            delay_before_restore_ms: 20,
        };
        let actions = plan_insert_actions(&inserter, Some(0), true);

        assert_eq!(
            actions,
            vec![
                InsertAction::SleepBeforePaste(10),
                InsertAction::Paste,
                InsertAction::SleepBeforeRestore(20),
                InsertAction::RestoreClipboard
            ]
        );
    }

    #[test]
    fn planned_insert_actions_keep_runtime_order() {
        let inserter = ClipboardInserter {
            restore_clipboard: true,
            delay_before_paste_ms: 15,
            delay_before_restore_ms: 25,
        };

        let actions = plan_insert_actions(&inserter, Some(42), true);

        assert_eq!(
            actions,
            vec![
                InsertAction::SleepBeforePaste(15),
                InsertAction::FocusWindow(42),
                InsertAction::SleepAfterFocus(40),
                InsertAction::Paste,
                InsertAction::SleepBeforeRestore(25),
                InsertAction::RestoreClipboard
            ]
        );
    }

    #[test]
    fn ctrl_v_key_sequence_presses_and_releases_control_v() {
        assert_eq!(
            ctrl_v_key_sequence(),
            [
                KeyAction::Press(Key::Control),
                KeyAction::Press(Key::V),
                KeyAction::Release(Key::V),
                KeyAction::Release(Key::Control)
            ]
        );
    }
}
