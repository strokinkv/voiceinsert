#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    TranscribePressed,
    TranslatePressed,
    Released,
}

pub trait HotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction>;
}

#[derive(Debug, Default)]
pub struct NoopHotkeyEvents;

impl HotkeyEvents for NoopHotkeyEvents {
    fn next_event(&mut self) -> Option<HotkeyAction> {
        None
    }
}
