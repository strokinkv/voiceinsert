use crate::hotkeys::matcher::normalize_hotkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    TranscribePressed,
    TranslatePressed,
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

#[cfg(windows)]
pub struct WindowsHotkeyService {
    registration: HotkeyRegistration,
}

#[cfg(windows)]
impl WindowsHotkeyService {
    pub fn register(transcription: &str, translation: &str) -> anyhow::Result<Self> {
        Ok(Self {
            registration: HotkeyRegistration::new(transcription, translation)?,
        })
    }

    pub fn registration(&self) -> &HotkeyRegistration {
        &self.registration
    }
}

#[cfg(windows)]
impl HotkeyEvents for WindowsHotkeyService {
    fn next_event(&mut self) -> Option<HotkeyAction> {
        // A low-level keyboard hook is implemented in the integration phase so Hold mode
        // can receive release events without consuming the user's keystrokes.
        None
    }
}

#[cfg(test)]
mod tests {
    use super::HotkeyRegistration;

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
}
