use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingMode {
    Toggle,
    Hold,
    SilenceTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLanguage {
    Russian,
    English,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub language: String,
    pub temperature: f64,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
            active_api_profile_id: String::new(),
            api_profiles: Vec::new(),
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
        self.hotkey = normalize_hotkey_or(&self.hotkey, "Ctrl+Space");
        self.translation_hotkey = normalize_hotkey_or(&self.translation_hotkey, "Alt+Y");

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
            .unwrap_or(&self.api_profiles[0])
    }

    fn ensure_profiles(&mut self) {
        if self.api_profiles.is_empty() {
            self.api_profiles.push(ai2npu_profile());
        }

        for profile in &mut self.api_profiles {
            let is_legacy_local_profile = profile.name.eq_ignore_ascii_case("Default")
                || profile.name.eq_ignore_ascii_case("wlast");
            if is_legacy_local_profile
                && profile
                    .base_url
                    .eq_ignore_ascii_case("http://127.0.0.1:9573")
            {
                profile.name = "ai2npu".to_string();
                profile.base_url = "http://localhost:9555".to_string();
            }
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
        model: String::new(),
        language: String::new(),
        temperature: 0.2,
        request_timeout_seconds: 120,
    }
}

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

fn normalize_hotkey_or(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}
