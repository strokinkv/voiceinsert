use std::collections::BTreeMap;

use voiceinsert::settings::AppSettings;

#[test]
fn app_paths_use_expected_file_names() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());

    assert!(paths.app_data_dir().ends_with("appdata\\VoiceInsert"));
    assert!(paths.settings_path().ends_with("settings.json"));
    assert!(paths.secrets_path().ends_with("api-key.dpapi"));
    assert!(
        paths
            .logs_dir()
            .ends_with("localappdata\\VoiceInsert\\Logs")
    );
}

#[test]
fn missing_settings_file_loads_ai2npu_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());

    let loaded = voiceinsert::settings::load_settings(&paths).unwrap();

    assert_eq!(loaded.active_profile().name, "ai2npu");
    assert_eq!(loaded.active_profile().base_url, "http://localhost:9555");
}

#[test]
fn settings_roundtrip_uses_settings_json() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());
    let settings = AppSettings::default().normalized();

    voiceinsert::settings::save_settings(&paths, &settings).unwrap();
    let loaded = voiceinsert::settings::load_settings(&paths).unwrap();
    let json = std::fs::read_to_string(paths.settings_path()).unwrap();

    assert_eq!(loaded.active_profile().name, "ai2npu");
    assert!(json.contains("\n  \"settingsVersion\""));
}

#[test]
fn secret_payload_roundtrip_preserves_profile_keys() {
    let mut keys = BTreeMap::new();
    keys.insert("profile-a".to_string(), "sk-a".to_string());
    keys.insert("profile-b".to_string(), "sk-b".to_string());

    let encoded = voiceinsert::secrets::serialize_keys(&keys).unwrap();
    let decoded = voiceinsert::secrets::deserialize_keys(&encoded).unwrap();

    assert_eq!(decoded, keys);
}

#[test]
fn legacy_plain_text_secret_deserializes_as_default_key() {
    let decoded = voiceinsert::secrets::deserialize_keys(b"sk-legacy").unwrap();

    assert_eq!(
        decoded.get("default").map(String::as_str),
        Some("sk-legacy")
    );
}

#[test]
fn api_keys_roundtrip_through_paths_secret_file() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());
    let mut keys = BTreeMap::new();
    keys.insert("ai2npu".to_string(), "local-key".to_string());
    keys.insert("groq".to_string(), "groq-key".to_string());

    voiceinsert::secrets::save_api_keys(&paths, &keys).unwrap();
    let loaded = voiceinsert::secrets::load_api_keys(&paths).unwrap();

    assert_eq!(loaded, keys);
    assert!(paths.secrets_path().exists());
}

#[test]
fn missing_api_key_file_loads_empty_map() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());

    let loaded = voiceinsert::secrets::load_api_keys(&paths).unwrap();

    assert!(loaded.is_empty());
}

#[test]
fn logging_init_creates_logs_dir_and_last_error_state_roundtrips() {
    let temp = tempfile::tempdir().unwrap();
    let paths = voiceinsert::paths::AppPaths::for_test(temp.path());

    voiceinsert::logging::init(&paths, "Debug").unwrap();
    let errors = voiceinsert::logging::LastErrorState::default();
    errors.set("failed");

    assert!(paths.logs_dir().exists());
    assert_eq!(errors.get().as_deref(), Some("failed"));
}

#[cfg(windows)]
#[test]
fn dpapi_protect_unprotect_roundtrip() {
    let protected = voiceinsert::secrets::protect(b"secret bytes").unwrap();
    let unprotected = voiceinsert::secrets::unprotect(&protected).unwrap();

    assert_eq!(unprotected, b"secret bytes");
}
