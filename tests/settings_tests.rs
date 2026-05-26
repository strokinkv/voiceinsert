use voiceinsert::settings::{AppLanguage, AppSettings, RecordingMode};

#[test]
fn localization_texts_match_language() {
    let russian = voiceinsert::i18n::texts(AppLanguage::Russian);
    let english = voiceinsert::i18n::texts(AppLanguage::English);

    assert_eq!(russian.settings, "Настройки");
    assert_eq!(russian.exit, "Выход");
    assert_eq!(english.settings, "Settings");
    assert_eq!(english.exit, "Exit");
}

#[test]
fn defaults_create_ai2npu_and_groq_profiles() {
    let settings = AppSettings::default().normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(settings.api_profiles[1].name, "groq");
    assert_eq!(
        settings.api_profiles[1].base_url,
        "https://api.groq.com/openai/"
    );
    assert_eq!(settings.api_profiles[1].model, "whisper-large-v3");
}

#[test]
fn default_active_profile_is_ai2npu_without_normalization() {
    let settings = AppSettings::default();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
}

#[test]
fn default_hotkeys_and_language_match_spec() {
    let settings = AppSettings::default().normalized();

    assert_eq!(settings.hotkey, "Ctrl+Space");
    assert_eq!(settings.translation_hotkey, "Alt+Y");
    assert_eq!(settings.ui_language, AppLanguage::Russian);
    assert_eq!(settings.recording_mode, RecordingMode::Toggle);
}

#[test]
fn partial_camel_case_settings_deserialize_and_normalize_profiles() {
    let settings: AppSettings = serde_json::from_str(
        r#"{
            "settingsVersion": 3,
            "hotkey": " Alt+Space "
        }"#,
    )
    .unwrap();

    let settings = settings.normalized();

    assert_eq!(settings.settings_version, 8);
    assert_eq!(settings.hotkey, "Alt+Space");
    assert_eq!(settings.active_profile().name, "ai2npu");
    assert!(
        settings
            .api_profiles
            .iter()
            .any(|profile| profile.name == "groq")
    );
}

#[test]
fn camel_case_settings_deserialize_profiles_and_active_profile() {
    let settings: AppSettings = serde_json::from_str(
        r#"{
            "settingsVersion": 8,
            "activeApiProfileId": "remote",
            "apiProfiles": [
                {
                    "id": "remote",
                    "name": "Remote",
                    "baseUrl": "https://example.test/openai/",
                    "model": "whisper",
                    "language": "en",
                    "temperature": 0.4,
                    "requestTimeoutSeconds": 45
                }
            ],
            "recordingMode": "Hold",
            "uiLanguage": "English"
        }"#,
    )
    .unwrap();

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().id, "remote");
    assert_eq!(
        settings.active_profile().base_url,
        "https://example.test/openai/"
    );
    assert_eq!(settings.active_profile().request_timeout_seconds, 45);
    assert_eq!(settings.recording_mode, RecordingMode::Hold);
    assert_eq!(settings.ui_language, AppLanguage::English);
}

#[test]
fn legacy_default_profile_migrates_to_ai2npu() {
    let mut settings = AppSettings::default();
    settings.api_profiles.clear();
    settings
        .api_profiles
        .push(voiceinsert::settings::ApiProfile {
            id: "legacy".to_string(),
            name: "Default".to_string(),
            base_url: "http://127.0.0.1:9573".to_string(),
            model: "legacy-model".to_string(),
            language: "ru".to_string(),
            temperature: 0.2,
            request_timeout_seconds: 120,
        });
    settings.active_api_profile_id = "legacy".to_string();

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(settings.active_profile().model, "legacy-model");
}

#[test]
fn legacy_wlast_profile_migrates_to_ai2npu() {
    let mut settings = AppSettings::default();
    settings.api_profiles.clear();
    settings
        .api_profiles
        .push(voiceinsert::settings::ApiProfile {
            id: "legacy-wlast".to_string(),
            name: "wlast".to_string(),
            base_url: "http://127.0.0.1:9573".to_string(),
            model: "legacy-model".to_string(),
            language: "ru".to_string(),
            temperature: 0.2,
            request_timeout_seconds: 120,
        });
    settings.active_api_profile_id = "legacy-wlast".to_string();

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
}

#[test]
fn numeric_settings_are_clamped() {
    let settings = AppSettings {
        temperature: 4.2,
        request_timeout_seconds: 1,
        silence_timeout_milliseconds: 50,
        max_recording_seconds: 0,
        ..AppSettings::default()
    };

    let settings = settings.normalized();

    assert_eq!(settings.temperature, 1.0);
    assert_eq!(settings.request_timeout_seconds, 5);
    assert_eq!(settings.silence_timeout_milliseconds, 100);
    assert_eq!(settings.max_recording_seconds, 1);
}
