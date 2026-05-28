use crate::hotkeys::matcher::normalize_hotkey;
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fs;
use std::sync::LazyLock;

use crate::paths::AppPaths;

pub const AI2NPU_DEFAULT_MODEL: &str = "openai/whisper-large-v3-turbo";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppSettings {
    pub settings_version: u32,
    pub active_api_profile_id: String,
    pub api_profiles: Vec<ApiProfile>,
    pub hotkey: String,
    pub translation_hotkey: String,
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
    pub temperature: f64,
    pub request_timeout_seconds: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            settings_version: 8,
            active_api_profile_id: "ai2npu".to_string(),
            api_profiles: vec![ai2npu_profile(), groq_profile()],
            hotkey: "Ctrl+Space".to_string(),
            translation_hotkey: "Alt+Y".to_string(),
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
            temperature: 0.2,
            request_timeout_seconds: 120,
        }
    }
}

impl AppSettings {
    pub fn normalized(mut self) -> Self {
        self.settings_version = 8;
        self.hotkey = normalize_hotkey(&self.hotkey).unwrap_or_else(|_| "Ctrl+Space".to_string());
        self.translation_hotkey =
            normalize_hotkey(&self.translation_hotkey).unwrap_or_else(|_| "Alt+Y".to_string());

        if self.hotkey.eq_ignore_ascii_case(&self.translation_hotkey) {
            self.translation_hotkey = if self.hotkey.eq_ignore_ascii_case("Alt+Y") {
                "Ctrl+Alt+Y".to_string()
            } else {
                "Alt+Y".to_string()
            };
        }

        self.temperature = clamp_f64((self.temperature * 10.0).round() / 10.0, 0.0, 1.0);
        self.request_timeout_seconds = self.request_timeout_seconds.clamp(5, 600);
        self.silence_threshold_percent = self.silence_threshold_percent.min(100);
        self.silence_timeout_milliseconds = self.silence_timeout_milliseconds.clamp(100, 30_000);
        self.max_recording_seconds = self.max_recording_seconds.clamp(1, 3600);
        self.delay_before_paste_milliseconds = self.delay_before_paste_milliseconds.min(5000);
        self.delay_before_clipboard_restore_milliseconds =
            self.delay_before_clipboard_restore_milliseconds.min(30_000);

        self.ensure_profiles();
        self
    }

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
            .any(|profile| profile.name.eq_ignore_ascii_case("ai2npu"))
        {
            self.api_profiles.push(ai2npu_profile());
        }

        if !self.api_profiles.iter().any(is_groq_profile) {
            self.api_profiles.push(groq_profile());
        }

        self.api_profiles.sort_by_key(|profile| {
            if profile.name.eq_ignore_ascii_case("ai2npu") {
                (0, profile.name.to_ascii_lowercase())
            } else if is_groq_profile(profile) {
                (1, profile.name.to_ascii_lowercase())
            } else {
                (2, profile.name.to_ascii_lowercase())
            }
        });

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

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

pub fn load_settings(paths: &AppPaths) -> anyhow::Result<AppSettings> {
    let path = paths.settings_path();
    if !path.exists() {
        return Ok(AppSettings::default().normalized());
    }

    let json = fs::read_to_string(path)?;
    Ok(serde_json::from_str::<AppSettings>(&json)?.normalized())
}

pub fn save_settings(paths: &AppPaths, settings: &AppSettings) -> anyhow::Result<()> {
    fs::create_dir_all(paths.app_data_dir())?;
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(paths.settings_path(), json)?;
    Ok(())
}
