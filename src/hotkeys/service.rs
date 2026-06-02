use crate::hotkeys::matcher::normalize_hotkey;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    TranscribePressed,
    TranslatePressed,
    CancelPressed,
    Released,
}

pub trait HotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRegistration {
    pub transcription: String,
    pub translation: String,
}

impl HotkeyRegistration {
    pub fn new(transcription: &str, translation: &str) -> anyhow::Result<Self> {
        let transcription = normalize_hotkey(transcription)?;
        let translation = normalize_hotkey(translation)?;
        if transcription.eq_ignore_ascii_case(&translation) {
            anyhow::bail!("Transcription and translation hotkeys must be different");
        }

        Ok(Self {
            transcription,
            translation,
        })
    }
}

#[derive(Debug, Default)]
pub struct NoopHotkeyEvents;

impl HotkeyEvents for NoopHotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction> {
        None
    }
}

pub struct GlobalHotkeyEvents {
    _manager: GlobalHotKeyManager,
    registration: HotkeyRegistration,
    transcription: HotKey,
    translation: HotKey,
}

impl GlobalHotkeyEvents {
    pub fn register(transcription: &str, translation: &str) -> anyhow::Result<Self> {
        let registration = HotkeyRegistration::new(transcription, translation)?;
        let [transcription_value, translation_value] = registered_hotkeys(&registration);
        let transcription = parse_global_hotkey(transcription_value)?;
        let translation = parse_global_hotkey(translation_value)?;
        let manager = GlobalHotKeyManager::new()?;

        manager.register(transcription)?;
        manager.register(translation)?;

        Ok(Self {
            _manager: manager,
            registration,
            transcription,
            translation,
        })
    }

    pub fn registration(&self) -> &HotkeyRegistration {
        &self.registration
    }
}

impl Drop for GlobalHotkeyEvents {
    fn drop(&mut self) {
        let _ = self._manager.unregister(self.transcription);
        let _ = self._manager.unregister(self.translation);
    }
}

impl HotkeyEvents for GlobalHotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction> {
        let event = GlobalHotKeyEvent::receiver().try_recv().ok()?;
        match event.state {
            HotKeyState::Pressed if event.id == self.transcription.id() => {
                Some(HotkeyAction::TranscribePressed)
            }
            HotKeyState::Pressed if event.id == self.translation.id() => {
                Some(HotkeyAction::TranslatePressed)
            }
            HotKeyState::Released
                if event.id == self.transcription.id() || event.id == self.translation.id() =>
            {
                Some(HotkeyAction::Released)
            }
            _ => None,
        }
    }
}

pub fn parse_global_hotkey(value: &str) -> anyhow::Result<HotKey> {
    let normalized = normalize_hotkey(value)?;
    Ok(normalized.parse()?)
}

fn registered_hotkeys(registration: &HotkeyRegistration) -> [&str; 2] {
    [
        registration.transcription.as_str(),
        registration.translation.as_str(),
    ]
}

#[cfg(test)]
mod tests {
    use super::{HotkeyRegistration, parse_global_hotkey, registered_hotkeys};

    #[test]
    fn registration_normalizes_hotkeys() {
        let registration = HotkeyRegistration::new("space+ctrl", "y+alt").unwrap();

        assert_eq!(registration.transcription, "Ctrl+Space");
        assert_eq!(registration.translation, "Alt+Y");
    }

    #[test]
    fn registration_rejects_duplicates() {
        assert!(HotkeyRegistration::new("Ctrl+Space", "Space+Ctrl").is_err());
    }

    #[test]
    fn normalized_hotkeys_parse_for_global_backend() {
        assert!(parse_global_hotkey("Ctrl+Space").is_ok());
        assert!(parse_global_hotkey("Alt+Y").is_ok());
        assert!(parse_global_hotkey("Ctrl+F12").is_ok());
    }

    #[test]
    fn registration_does_not_include_global_escape_hotkey() {
        let registration = HotkeyRegistration::new("Ctrl+Space", "Alt+Y").unwrap();
        let hotkeys = registered_hotkeys(&registration);

        assert_eq!(hotkeys, ["Ctrl+Space", "Alt+Y"]);
        assert!(
            !hotkeys
                .iter()
                .any(|hotkey| hotkey.eq_ignore_ascii_case("Esc"))
        );
    }
}
