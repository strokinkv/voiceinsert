use crate::hotkeys::matcher::normalize_hotkey;
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fs;
use std::sync::LazyLock;

use crate::paths::AppPaths;

/// Default ai2npu/OpenAI-compatible transcription model used when a profile model is empty.
pub const AI2NPU_DEFAULT_MODEL: &str = "openai/whisper-large-v3-turbo";

/// Recording stop policy controlled by the hotkey mode setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RecordingMode {
    Toggle,
    Hold,
    SilenceTimeout,
}

impl<'de> Deserialize<'de> for RecordingMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_enum(
            deserializer,
            &["Toggle", "Hold", "SilenceTimeout"],
            |value| match value {
                0 => Some(Self::Toggle),
                1 => Some(Self::Hold),
                2 => Some(Self::SilenceTimeout),
                _ => None,
            },
            |value| match value {
                "Toggle" => Some(Self::Toggle),
                "Hold" => Some(Self::Hold),
                "SilenceTimeout" => Some(Self::SilenceTimeout),
                _ => None,
            },
        )
    }
}

/// Settings UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AppLanguage {
    Russian,
    English,
}

impl<'de> Deserialize<'de> for AppLanguage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_enum(
            deserializer,
            &["Russian", "English"],
            |value| match value {
                0 => Some(Self::Russian),
                1 => Some(Self::English),
                _ => None,
            },
            |value| match value {
                "Russian" => Some(Self::Russian),
                "English" => Some(Self::English),
                _ => None,
            },
        )
    }
}

struct EnumVisitor<T> {
    expected: &'static [&'static str],
    from_u64: fn(u64) -> Option<T>,
    from_str: fn(&str) -> Option<T>,
}

impl<T> Visitor<'_> for EnumVisitor<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "one of {}", self.expected.join(", "))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        (self.from_u64)(value).ok_or_else(|| {
            E::invalid_value(
                Unexpected::Unsigned(value),
                &"a supported enum numeric value",
            )
        })
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let value = u64::try_from(value)
            .map_err(|_| E::invalid_value(Unexpected::Signed(value), &"a non-negative value"))?;
        self.visit_u64(value)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        (self.from_str)(value).ok_or_else(|| E::unknown_variant(value, self.expected))
    }
}

fn deserialize_enum<'de, D, T>(
    deserializer: D,
    expected: &'static [&'static str],
    from_u64: fn(u64) -> Option<T>,
    from_str: fn(&str) -> Option<T>,
) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(EnumVisitor {
        expected,
        from_u64,
        from_str,
    })
}

/// OpenAI-compatible audio API profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ApiProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub language: String,
    pub temperature: f64,
    pub request_timeout_seconds: u64,
}

impl Default for ApiProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base_url: String::new(),
            model: String::new(),
            language: String::new(),
            temperature: 0.2,
            request_timeout_seconds: 120,
        }
    }
}

/// Persisted application settings loaded from `settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub settings_version: u32,
    pub active_api_profile_id: String,
    pub api_profiles: Vec<ApiProfile>,
    pub hotkey: String,
    pub recording_mode: RecordingMode,
    pub silence_threshold_percent: u8,
    pub silence_timeout_milliseconds: u64,
    pub max_recording_seconds: u64,
    pub start_with_windows: bool,
    pub ui_language: AppLanguage,
    pub launch_minimized_to_tray: bool,
    pub show_floating_recording_window: bool,
    pub enable_sounds: bool,
    pub restore_clipboard_content: bool,
    pub delay_before_paste_milliseconds: u64,
    pub delay_before_clipboard_restore_milliseconds: u64,
    pub log_level: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            settings_version: 8,
            active_api_profile_id: "ai2npu".to_string(),
            api_profiles: vec![ai2npu_profile(), groq_profile()],
            hotkey: "Ctrl+Space".to_string(),
            recording_mode: RecordingMode::Toggle,
            silence_threshold_percent: 4,
            silence_timeout_milliseconds: 1200,
            max_recording_seconds: 120,
            start_with_windows: false,
            ui_language: AppLanguage::Russian,
            launch_minimized_to_tray: true,
            show_floating_recording_window: true,
            enable_sounds: true,
            restore_clipboard_content: true,
            delay_before_paste_milliseconds: 80,
            delay_before_clipboard_restore_milliseconds: 300,
            log_level: "Information".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AppSettingsFile {
    settings_version: u32,
    active_api_profile_id: String,
    api_profiles: Option<Vec<ApiProfile>>,
    hotkey: String,
    recording_mode: RecordingMode,
    silence_threshold_percent: u8,
    silence_timeout_milliseconds: u64,
    max_recording_seconds: u64,
    start_with_windows: bool,
    ui_language: AppLanguage,
    launch_minimized_to_tray: bool,
    show_floating_recording_window: bool,
    enable_sounds: bool,
    restore_clipboard_content: bool,
    delay_before_paste_milliseconds: u64,
    delay_before_clipboard_restore_milliseconds: u64,
    log_level: String,
    temperature: Option<f64>,
    request_timeout_seconds: Option<u64>,
}

impl Default for AppSettingsFile {
    fn default() -> Self {
        let defaults = AppSettings::default();
        Self {
            settings_version: defaults.settings_version,
            active_api_profile_id: defaults.active_api_profile_id,
            api_profiles: None,
            hotkey: defaults.hotkey,
            recording_mode: defaults.recording_mode,
            silence_threshold_percent: defaults.silence_threshold_percent,
            silence_timeout_milliseconds: defaults.silence_timeout_milliseconds,
            max_recording_seconds: defaults.max_recording_seconds,
            start_with_windows: defaults.start_with_windows,
            ui_language: defaults.ui_language,
            launch_minimized_to_tray: defaults.launch_minimized_to_tray,
            show_floating_recording_window: defaults.show_floating_recording_window,
            enable_sounds: defaults.enable_sounds,
            restore_clipboard_content: defaults.restore_clipboard_content,
            delay_before_paste_milliseconds: defaults.delay_before_paste_milliseconds,
            delay_before_clipboard_restore_milliseconds: defaults
                .delay_before_clipboard_restore_milliseconds,
            log_level: defaults.log_level,
            temperature: None,
            request_timeout_seconds: None,
        }
    }
}

impl AppSettingsFile {
    fn into_settings(self) -> AppSettings {
        let had_profiles = self
            .api_profiles
            .as_ref()
            .is_some_and(|profiles| !profiles.is_empty());
        let legacy_temperature = self.temperature;
        let legacy_timeout = self.request_timeout_seconds;

        let mut settings = AppSettings {
            settings_version: self.settings_version,
            active_api_profile_id: self.active_api_profile_id,
            api_profiles: self.api_profiles.unwrap_or_default(),
            hotkey: self.hotkey,
            recording_mode: self.recording_mode,
            silence_threshold_percent: self.silence_threshold_percent,
            silence_timeout_milliseconds: self.silence_timeout_milliseconds,
            max_recording_seconds: self.max_recording_seconds,
            start_with_windows: self.start_with_windows,
            ui_language: self.ui_language,
            launch_minimized_to_tray: self.launch_minimized_to_tray,
            show_floating_recording_window: self.show_floating_recording_window,
            enable_sounds: self.enable_sounds,
            restore_clipboard_content: self.restore_clipboard_content,
            delay_before_paste_milliseconds: self.delay_before_paste_milliseconds,
            delay_before_clipboard_restore_milliseconds: self
                .delay_before_clipboard_restore_milliseconds,
            log_level: self.log_level,
        }
        .normalized();

        if !had_profiles && let Some(profile) = settings.api_profiles.first_mut() {
            if let Some(temperature) = legacy_temperature {
                profile.temperature = temperature;
            }
            if let Some(timeout) = legacy_timeout {
                profile.request_timeout_seconds = timeout;
            }
        }

        settings.normalized()
    }
}

impl<'de> Deserialize<'de> for AppSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(AppSettingsFile::deserialize(deserializer)?.into_settings())
    }
}

impl AppSettings {
    /// Applies defaults, clamps unsafe values, normalizes hotkeys, and ensures built-in profiles exist.
    pub fn normalized(mut self) -> Self {
        self.settings_version = 8;
        self.hotkey = normalize_hotkey(&self.hotkey).unwrap_or_else(|_| "Ctrl+Space".to_string());

        self.silence_threshold_percent = self.silence_threshold_percent.min(100);
        self.silence_timeout_milliseconds = self.silence_timeout_milliseconds.clamp(100, 30_000);
        self.max_recording_seconds = self.max_recording_seconds.clamp(1, 3600);
        self.launch_minimized_to_tray = true;
        self.show_floating_recording_window = true;
        self.delay_before_paste_milliseconds = self.delay_before_paste_milliseconds.min(5000);
        self.delay_before_clipboard_restore_milliseconds =
            self.delay_before_clipboard_restore_milliseconds.min(30_000);
        self.log_level = match self.log_level.as_str() {
            "Debug" => "Debug",
            "Warning" => "Warning",
            "Error" => "Error",
            _ => "Information",
        }
        .to_string();

        for profile in &mut self.api_profiles {
            profile.temperature = (profile.temperature * 10.0).round() / 10.0;
            profile.temperature = profile.temperature.clamp(0.0, 1.0);
            profile.request_timeout_seconds = profile.request_timeout_seconds.clamp(5, 600);
        }

        self.ensure_profiles();
        self
    }

    /// Returns the active API profile, falling back to the first or default ai2npu profile.
    pub fn active_profile(&self) -> &ApiProfile {
        self.api_profiles
            .iter()
            .find(|profile| profile.id == self.active_api_profile_id)
            .or_else(|| self.api_profiles.first())
            .unwrap_or(&DEFAULT_AI2NPU_PROFILE)
    }

    fn ensure_profiles(&mut self) {
        if self.api_profiles.is_empty() {
            self.api_profiles.push(ai2npu_profile());
        }

        if !self
            .api_profiles
            .iter()
            .any(|profile| profile.id == "ai2npu" || profile.name.eq_ignore_ascii_case("ai2npu"))
        {
            self.api_profiles.push(ai2npu_profile());
        }

        if !self.api_profiles.iter().any(is_groq_profile) {
            self.api_profiles.push(groq_profile());
        }

        if self.active_api_profile_id.is_empty()
            || !self
                .api_profiles
                .iter()
                .any(|profile| profile.id == self.active_api_profile_id)
        {
            self.active_api_profile_id = self
                .api_profiles
                .iter()
                .find(|profile| profile.name.eq_ignore_ascii_case("ai2npu"))
                .map(|profile| profile.id.clone())
                .unwrap_or_else(|| self.api_profiles[0].id.clone());
        }

        self.ensure_unique_profile_names();
    }

    fn ensure_unique_profile_names(&mut self) {
        let mut used_names = Vec::<String>::new();
        for (index, profile) in self.api_profiles.iter_mut().enumerate() {
            let base_name = if profile.name.trim().is_empty() {
                if profile.id.trim().is_empty() {
                    format!("profile-{}", index + 1)
                } else {
                    profile.id.trim().to_string()
                }
            } else {
                profile.name.trim().to_string()
            };

            let mut candidate = base_name.clone();
            let mut suffix = 2;
            while used_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&candidate))
            {
                candidate = format!("{base_name} ({suffix})");
                suffix += 1;
            }

            profile.name = candidate.clone();
            used_names.push(candidate);
        }
    }
}

fn ai2npu_profile() -> ApiProfile {
    ApiProfile {
        id: "ai2npu".to_string(),
        name: "ai2npu".to_string(),
        base_url: "http://localhost:9555".to_string(),
        model: AI2NPU_DEFAULT_MODEL.to_string(),
        language: String::new(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    }
}

static DEFAULT_AI2NPU_PROFILE: LazyLock<ApiProfile> = LazyLock::new(ai2npu_profile);

fn groq_profile() -> ApiProfile {
    ApiProfile {
        id: "groq".to_string(),
        name: "groq".to_string(),
        base_url: "https://api.groq.com/openai/".to_string(),
        model: "whisper-large-v3".to_string(),
        language: String::new(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    }
}

fn is_groq_profile(profile: &ApiProfile) -> bool {
    profile.name.eq_ignore_ascii_case("groq")
        || profile.base_url.to_ascii_lowercase().contains("groq.com")
}

/// Loads settings from disk, returning normalized defaults when no settings file exists.
pub fn load_settings(paths: &AppPaths) -> anyhow::Result<AppSettings> {
    let path = paths.settings_path();
    if !path.exists() {
        return Ok(AppSettings::default().normalized());
    }

    let json = fs::read_to_string(path)?;
    settings_from_json(&json)
}

/// Deserializes settings JSON, including legacy top-level profile values.
pub fn settings_from_json(json: &str) -> anyhow::Result<AppSettings> {
    Ok(serde_json::from_str::<AppSettings>(json)?)
}

/// Saves settings as pretty JSON in the configured app data directory.
pub fn save_settings(paths: &AppPaths, settings: &AppSettings) -> anyhow::Result<()> {
    fs::create_dir_all(paths.app_data_dir())?;
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(paths.settings_path(), json)?;
    Ok(())
}
