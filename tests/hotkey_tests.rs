use voiceinsert::hotkeys::matcher::{normalize_hotkey, validate_pair};

#[test]
fn normalizes_modifier_order() {
    assert_eq!(normalize_hotkey("space+ctrl").unwrap(), "Ctrl+Space");
    assert_eq!(normalize_hotkey("Y+Alt").unwrap(), "Alt+Y");
}

#[test]
fn rejects_hotkeys_without_modifier() {
    assert!(normalize_hotkey("Space").is_err());
}

#[test]
fn rejects_duplicate_transcription_and_translation_hotkeys() {
    assert!(validate_pair("Ctrl+Space", "Ctrl+Space").is_err());
}
