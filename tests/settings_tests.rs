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
    assert_eq!(settings.active_profile().temperature, 0.2);
    assert_eq!(settings.active_profile().request_timeout_seconds, 120);
    assert_eq!(
        settings.active_profile().model,
        "openai/whisper-large-v3-turbo"
    );
    assert_eq!(settings.api_profiles[1].name, "groq");
    assert_eq!(
        settings.api_profiles[1].base_url,
        "https://api.groq.com/openai/"
    );
    assert_eq!(settings.api_profiles[1].temperature, 0.2);
    assert_eq!(settings.api_profiles[1].request_timeout_seconds, 120);
    assert_eq!(settings.api_profiles[1].model, "whisper-large-v3");
}

#[test]
fn default_active_profile_is_ai2npu_without_normalization() {
    let settings = AppSettings::default();

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(
        settings.active_profile().model,
        "openai/whisper-large-v3-turbo"
    );
}

#[test]
fn active_profile_falls_back_to_ai2npu_when_profiles_are_empty() {
    let settings = AppSettings {
        api_profiles: Vec::new(),
        active_api_profile_id: "missing".to_string(),
        ..AppSettings::default()
    };

    assert_eq!(settings.active_profile().name, "ai2npu");
    assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
    assert_eq!(
        settings.active_profile().model,
        "openai/whisper-large-v3-turbo"
    );
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
fn invalid_hotkeys_fall_back_to_defaults() {
    let settings = AppSettings {
        hotkey: "Y".to_string(),
        translation_hotkey: "Unknown+Key".to_string(),
        ..AppSettings::default()
    }
    .normalized();

    assert_eq!(settings.hotkey, "Ctrl+Space");
    assert_eq!(settings.translation_hotkey, "Alt+Y");
}

#[test]
fn hotkeys_are_canonicalized_during_normalization() {
    let settings = AppSettings {
        hotkey: "space+ctrl".to_string(),
        translation_hotkey: "y+alt".to_string(),
        ..AppSettings::default()
    }
    .normalized();

    assert_eq!(settings.hotkey, "Ctrl+Space");
    assert_eq!(settings.translation_hotkey, "Alt+Y");
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
fn legacy_top_level_profile_values_migrate_when_profiles_are_missing() {
    let json = r#"{
        "settingsVersion": 7,
        "activeApiProfileId": "",
        "temperature": 0.7,
        "requestTimeoutSeconds": 45
    }"#;

    let settings = voiceinsert::settings::settings_from_json(json).unwrap();

    assert_eq!(settings.active_profile().temperature, 0.7);
    assert_eq!(settings.active_profile().request_timeout_seconds, 45);
}

#[test]
fn camel_case_settings_deserialize_profiles_and_active_profile() {
    let settings: AppSettings = serde_json::from_str(
        r#"{
            "settingsVersion": 8,
            "activeApiProfileId": "remote",
            "temperature": 0.9,
            "requestTimeoutSeconds": 999,
            "inputDeviceIndex": 3,
            "apiProfiles": [
                {
                    "id": "remote",
                    "name": "Remote",
                    "baseUrl": "https://example.test/openai/",
                    "model": "whisper",
                    "language": "en",
                    "temperature": 4.2,
                    "requestTimeoutSeconds": 1
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
    assert_eq!(settings.active_profile().temperature, 1.0);
    assert_eq!(settings.active_profile().request_timeout_seconds, 5);
    assert_eq!(settings.recording_mode, RecordingMode::Hold);
    assert_eq!(settings.ui_language, AppLanguage::English);
}

#[test]
fn profile_settings_are_clamped_during_normalization() {
    let settings = AppSettings {
        api_profiles: vec![voiceinsert::settings::ApiProfile {
            id: "custom".to_string(),
            name: "custom".to_string(),
            base_url: "https://example.test/openai/".to_string(),
            model: "whisper".to_string(),
            language: "en".to_string(),
            temperature: 4.2,
            request_timeout_seconds: 1,
        }],
        active_api_profile_id: "custom".to_string(),
        silence_timeout_milliseconds: 50,
        max_recording_seconds: 0,
        ..AppSettings::default()
    };

    let settings = settings.normalized();

    assert_eq!(settings.active_profile().temperature, 1.0);
    assert_eq!(settings.active_profile().request_timeout_seconds, 5);
    assert_eq!(settings.silence_timeout_milliseconds, 100);
    assert_eq!(settings.max_recording_seconds, 1);
}

#[test]
fn normalization_preserves_existing_profile_order_after_rename() {
    let settings = AppSettings {
        active_api_profile_id: "ai2npu".to_string(),
        api_profiles: vec![
            voiceinsert::settings::ApiProfile {
                id: "custom".to_string(),
                name: "Custom".to_string(),
                base_url: "https://example.test/openai/".to_string(),
                model: "whisper".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
            voiceinsert::settings::ApiProfile {
                id: "ai2npu".to_string(),
                name: "Local NPU".to_string(),
                base_url: "http://localhost:9555".to_string(),
                model: "openai/whisper-large-v3-turbo".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
            voiceinsert::settings::ApiProfile {
                id: "groq".to_string(),
                name: "groq".to_string(),
                base_url: "https://api.groq.com/openai/".to_string(),
                model: "whisper-large-v3".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
        ],
        ..AppSettings::default()
    }
    .normalized();

    let ids = settings
        .api_profiles
        .iter()
        .map(|profile| profile.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, ["custom", "ai2npu", "groq"]);
    assert_eq!(settings.active_profile().name, "Local NPU");
}

#[test]
fn normalization_makes_profile_names_visible_and_unique() {
    let settings = AppSettings {
        active_api_profile_id: "one".to_string(),
        api_profiles: vec![
            voiceinsert::settings::ApiProfile {
                id: "one".to_string(),
                name: "Duplicate".to_string(),
                base_url: "https://one.example/openai/".to_string(),
                model: "whisper".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
            voiceinsert::settings::ApiProfile {
                id: "two".to_string(),
                name: " duplicate ".to_string(),
                base_url: "https://two.example/openai/".to_string(),
                model: "whisper".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
            voiceinsert::settings::ApiProfile {
                id: "blank-id".to_string(),
                name: "   ".to_string(),
                base_url: "https://blank.example/openai/".to_string(),
                model: "whisper".to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            },
        ],
        ..AppSettings::default()
    }
    .normalized();

    let names = settings
        .api_profiles
        .iter()
        .map(|profile| profile.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names[0..3], ["Duplicate", "duplicate (2)", "blank-id"]);
}
