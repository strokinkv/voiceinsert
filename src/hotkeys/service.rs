use crate::hotkeys::matcher::normalize_hotkey;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    TranscribePressed,
    Released,
    CopilotPressed,
    CopilotReleased,
}

pub trait HotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRegistration {
    pub transcription: String,
}

impl HotkeyRegistration {
    pub fn new(transcription: &str) -> anyhow::Result<Self> {
        let transcription = normalize_hotkey(transcription)?;

        Ok(Self { transcription })
    }
}

pub struct GlobalHotkeyEvents {
    _manager: GlobalHotKeyManager,
    registration: HotkeyRegistration,
    transcription: Vec<HotKey>,
    copilot_rx: Receiver<CopilotKeyEvent>,
    _copilot_hook: CopilotHotkeyHook,
}

impl GlobalHotkeyEvents {
    pub fn register(transcription: &str) -> anyhow::Result<Self> {
        let registration = HotkeyRegistration::new(transcription)?;
        let (copilot_hook, copilot_rx) = CopilotHotkeyHook::new()?;
        let transcription = registered_hotkeys(&registration)
            .into_iter()
            .map(parse_global_hotkey)
            .collect::<anyhow::Result<Vec<_>>>()?;
        let manager = GlobalHotKeyManager::new()?;

        for hotkey in &transcription {
            manager.register(*hotkey)?;
        }

        Ok(Self {
            _manager: manager,
            registration,
            transcription,
            copilot_rx,
            _copilot_hook: copilot_hook,
        })
    }

    pub fn registration(&self) -> &HotkeyRegistration {
        &self.registration
    }
}

impl Drop for GlobalHotkeyEvents {
    fn drop(&mut self) {
        for hotkey in &self.transcription {
            let _ = self._manager.unregister(*hotkey);
        }
    }
}

impl HotkeyEvents for GlobalHotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction> {
        if let Ok(event) = self.copilot_rx.try_recv() {
            return Some(match event {
                CopilotKeyEvent::Pressed => HotkeyAction::CopilotPressed,
                CopilotKeyEvent::Released => HotkeyAction::CopilotReleased,
            });
        }

        let event = GlobalHotKeyEvent::receiver().try_recv().ok()?;
        let is_transcription = self
            .transcription
            .iter()
            .any(|hotkey| event.id == hotkey.id());
        match event.state {
            HotKeyState::Pressed if is_transcription => Some(HotkeyAction::TranscribePressed),
            HotKeyState::Released if is_transcription => Some(HotkeyAction::Released),
            _ => None,
        }
    }
}

pub fn parse_global_hotkey(value: &str) -> anyhow::Result<HotKey> {
    let normalized = normalize_hotkey(value)?;
    Ok(global_hotkey_value(&normalized).parse()?)
}

fn registered_hotkeys(registration: &HotkeyRegistration) -> Vec<&str> {
    if registration.transcription == "Shift+Win+F23" {
        Vec::new()
    } else {
        vec![registration.transcription.as_str()]
    }
}

pub(crate) fn is_copilot_hotkey(value: &str) -> bool {
    value == "F23" || value == "Shift+Win+F23"
}

fn global_hotkey_value(value: &str) -> String {
    value
        .split('+')
        .map(|part| if part == "Win" { "Super" } else { part })
        .collect::<Vec<_>>()
        .join("+")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopilotKeyEvent {
    Pressed,
    Released,
}

#[cfg(windows)]
struct CopilotHookState {
    tx: Option<(u64, Sender<CopilotKeyEvent>)>,
    active: bool,
}

#[cfg(windows)]
static COPILOT_HOOK_STATE: std::sync::Mutex<CopilotHookState> =
    std::sync::Mutex::new(CopilotHookState {
        tx: None,
        active: false,
    });

#[cfg(windows)]
static NEXT_COPILOT_HOOK_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[cfg(windows)]
struct CopilotHotkeyHook {
    id: u64,
    hook: isize,
}

#[cfg(windows)]
impl CopilotHotkeyHook {
    fn new() -> anyhow::Result<(Self, Receiver<CopilotKeyEvent>)> {
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_KEYBOARD_LL};

        let (tx, rx) = std::sync::mpsc::channel();
        let id = NEXT_COPILOT_HOOK_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        {
            let mut state = COPILOT_HOOK_STATE
                .lock()
                .map_err(|_| anyhow::anyhow!("Copilot hotkey hook state is poisoned"))?;
            state.tx = Some((id, tx));
            state.active = false;
        }

        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(copilot_keyboard_proc), None, 0) }
                .map_err(|error| {
                    anyhow::anyhow!("failed to install Copilot hotkey hook: {error}")
                })?;

        Ok((
            Self {
                id,
                hook: hook.0 as isize,
            },
            rx,
        ))
    }
}

#[cfg(windows)]
impl Drop for CopilotHotkeyHook {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::{HHOOK, UnhookWindowsHookEx};

        if let Ok(mut state) = COPILOT_HOOK_STATE.lock()
            && state.tx.as_ref().is_some_and(|(id, _)| *id == self.id)
        {
            state.tx = None;
            state.active = false;
        }
        let hook = HHOOK(self.hook as *mut core::ffi::c_void);
        let _ = unsafe { UnhookWindowsHookEx(hook) };
    }
}

#[cfg(windows)]
unsafe extern "system" fn copilot_keyboard_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_F23, VK_LWIN, VK_RWIN, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    };

    if code == HC_ACTION as i32 {
        let event = unsafe { *(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if event.vkCode == u32::from(VK_F23.0) {
            let message = wparam.0 as u32;
            let is_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let is_up = message == WM_KEYUP || message == WM_SYSKEYUP;
            if is_down || is_up {
                let shift_down = unsafe { GetAsyncKeyState(i32::from(VK_SHIFT.0)) } < 0;
                let win_down = unsafe { GetAsyncKeyState(i32::from(VK_LWIN.0)) } < 0
                    || unsafe { GetAsyncKeyState(i32::from(VK_RWIN.0)) } < 0;

                if let Ok(mut state) = COPILOT_HOOK_STATE.lock() {
                    if is_down && shift_down && win_down && !state.active {
                        state.active = true;
                        if let Some((_, tx)) = &state.tx {
                            let _ = tx.send(CopilotKeyEvent::Pressed);
                        }
                    } else if is_up && state.active {
                        state.active = false;
                        if let Some((_, tx)) = &state.tx {
                            let _ = tx.send(CopilotKeyEvent::Released);
                        }
                    }
                }
            }
        }
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(not(windows))]
struct CopilotHotkeyHook;

#[cfg(not(windows))]
impl CopilotHotkeyHook {
    fn new() -> anyhow::Result<(Self, Receiver<CopilotKeyEvent>)> {
        let (_tx, rx) = std::sync::mpsc::channel();
        Ok((Self, rx))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HotkeyRegistration, global_hotkey_value, is_copilot_hotkey, parse_global_hotkey,
        registered_hotkeys,
    };

    #[test]
    fn registration_normalizes_hotkeys() {
        let registration = HotkeyRegistration::new("space+ctrl").unwrap();

        assert_eq!(registration.transcription, "Ctrl+Space");
    }

    #[test]
    fn normalized_hotkeys_parse_for_global_backend() {
        assert!(parse_global_hotkey("Ctrl+Space").is_ok());
        assert!(parse_global_hotkey("Alt+Y").is_ok());
        assert!(parse_global_hotkey("Ctrl+F12").is_ok());
        assert!(parse_global_hotkey("F23").is_ok());
        assert!(parse_global_hotkey("Shift+Win+F23").is_ok());
        assert!(parse_global_hotkey("Space").is_ok());
    }

    #[test]
    fn global_backend_uses_super_for_windows_modifier() {
        assert_eq!(global_hotkey_value("Shift+Win+F23"), "Shift+Super+F23");
    }

    #[test]
    fn plain_f23_also_registers_copilot_chord() {
        let registration = HotkeyRegistration::new("F23").unwrap();
        let hotkeys = registered_hotkeys(&registration);

        assert_eq!(hotkeys, vec!["F23"]);
    }

    #[test]
    fn copilot_chord_uses_low_level_hook_registration() {
        let registration = HotkeyRegistration::new("Shift+Win+F23").unwrap();
        let hotkeys = registered_hotkeys(&registration);

        assert!(hotkeys.is_empty());
        assert!(is_copilot_hotkey(&registration.transcription));
    }

    #[test]
    fn registration_does_not_include_global_escape_hotkey() {
        let registration = HotkeyRegistration::new("Ctrl+Space").unwrap();
        let hotkeys = registered_hotkeys(&registration);

        assert_eq!(hotkeys, vec!["Ctrl+Space"]);
        assert!(
            !hotkeys
                .iter()
                .any(|hotkey| hotkey.eq_ignore_ascii_case("Esc"))
        );
    }
}
