use super::{AppRuntime, non_empty_str};
use crate::clipboard::ClipboardInserter;
use crate::hotkeys::service::GlobalHotkeyEvents;
use crate::settings::{AI2NPU_DEFAULT_MODEL, ApiProfile, AppLanguage, AppSettings, RecordingMode};
use crate::ui::{SettingsEdit, UiCommand};
use std::collections::BTreeMap;
use std::time::Duration;

impl AppRuntime {
    pub(super) fn handle_ui_command(&mut self, command: UiCommand) -> anyhow::Result<()> {
        match command {
            UiCommand::SettingsChanged => match self.save_settings_from_ui() {
                Ok(models_source_changed) => {
                    self.ui
                        .set_status(settings_saved_status(self.settings.ui_language));
                    if models_source_changed {
                        self.queue_model_load_for_active_profile()?;
                    }
                }
                Err(error) => {
                    let message = super::state::log_error_message(&error);
                    tracing::warn!(message = %message, "settings autosave skipped");
                    self.ui.set_status(message);
                }
            },
            UiCommand::AddApiProfile => {
                let new_profile = next_api_profile(&self.settings.api_profiles);
                self.settings.active_api_profile_id = new_profile.id.clone();
                self.settings.api_profiles.push(new_profile);
                self.settings = self.settings.clone().normalized();
                self.http = http_client_for_profile(self.settings.active_profile())?;
                crate::settings::save_settings(&self.paths, &self.settings)?;
                self.reopen_settings();
                self.ui
                    .set_status(api_profile_added_status(self.settings.ui_language));
                self.queue_model_load_for_active_profile()?;
            }
            UiCommand::DeleteApiProfile => {
                let active_profile_id = self.settings.active_api_profile_id.clone();
                if self.settings.api_profiles.len() > 1 {
                    self.settings
                        .api_profiles
                        .retain(|profile| profile.id != active_profile_id);
                    self.api_keys.remove(&active_profile_id);
                    self.settings.active_api_profile_id = self
                        .settings
                        .api_profiles
                        .first()
                        .map(|profile| profile.id.clone())
                        .unwrap_or_default();
                    self.settings = self.settings.clone().normalized();
                    self.http = http_client_for_profile(self.settings.active_profile())?;
                    crate::settings::save_settings(&self.paths, &self.settings)?;
                    crate::secrets::save_api_keys(&self.paths, &self.api_keys)?;
                    self.reopen_settings();
                    self.ui
                        .set_status(api_profile_deleted_status(self.settings.ui_language));
                    self.queue_model_load_for_active_profile()?;
                } else {
                    self.ui
                        .set_status(api_profile_required_status(self.settings.ui_language));
                }
            }
            UiCommand::SelectApiProfile(profile_name) => {
                if let Some(profile_id) = profile_id_by_name(&self.settings, &profile_name)
                    && profile_id != self.settings.active_api_profile_id
                {
                    self.settings.active_api_profile_id = profile_id;
                    self.settings = self.settings.clone().normalized();
                    self.http = http_client_for_profile(self.settings.active_profile())?;
                    crate::settings::save_settings(&self.paths, &self.settings)?;
                    self.reopen_settings();
                    self.ui
                        .set_status(api_profile_selected_status(self.settings.ui_language));
                    self.queue_model_load_for_active_profile()?;
                }
            }
            UiCommand::OpenLogsFolder => {
                open_folder(&self.paths.logs_dir())?;
            }
        }

        Ok(())
    }

    fn save_settings_from_ui(&mut self) -> anyhow::Result<bool> {
        let Some(edit) = self.ui.settings_edit() else {
            return Ok(false);
        };

        crate::hotkeys::matcher::validate_pair(
            &edit.transcription_hotkey,
            &edit.translation_hotkey,
        )?;
        crate::api::endpoints::endpoint(&edit.base_url, "/v1/models")?;

        let old_hotkey = self.settings.hotkey.clone();
        let old_translation_hotkey = self.settings.translation_hotkey.clone();
        let old_language = self.settings.ui_language;
        let active_profile_id = self.settings.active_api_profile_id.clone();
        let old_profile = self.settings.active_profile().clone();
        let old_api_key = api_key_for_profile(&self.api_keys, &active_profile_id).to_string();
        let api_key = edit.api_key.trim().to_string();

        self.settings = apply_settings_edit(self.settings.clone(), edit).normalized();
        if api_key.is_empty() {
            self.api_keys.remove(&active_profile_id);
        } else {
            self.api_keys
                .insert(active_profile_id.clone(), api_key.clone());
        }
        crate::secrets::save_api_keys(&self.paths, &self.api_keys)?;

        if self.settings.hotkey != old_hotkey
            || self.settings.translation_hotkey != old_translation_hotkey
        {
            self.hotkeys = GlobalHotkeyEvents::register(
                &self.settings.hotkey,
                &self.settings.translation_hotkey,
            )?;
        }
        if self.settings.ui_language != old_language {
            self.tray.set_language(self.settings.ui_language);
        }
        self.clipboard = ClipboardInserter {
            restore_clipboard: self.settings.restore_clipboard_content,
            delay_before_paste_ms: self.settings.delay_before_paste_milliseconds,
            delay_before_restore_ms: self.settings.delay_before_clipboard_restore_milliseconds,
        };
        self.sounds.enabled = self.settings.enable_sounds;
        self.state = super::state::VoiceInsertState::new(
            self.settings.recording_mode,
            f32::from(self.settings.silence_threshold_percent) / 100.0,
            self.settings.silence_timeout_milliseconds,
            self.settings.max_recording_seconds.saturating_mul(1000),
        );
        self.http = http_client_for_profile(self.settings.active_profile())?;
        #[cfg(windows)]
        crate::autostart::set_enabled(self.settings.start_with_windows, &std::env::current_exe()?)?;

        crate::settings::save_settings(&self.paths, &self.settings)?;
        self.ui.set_profile_metadata(&self.settings);

        let new_profile = self.settings.active_profile();
        Ok(old_profile.base_url != new_profile.base_url
            || profile_requires_http_rebuild(&old_profile, new_profile)
            || old_api_key != api_key)
    }

    fn reopen_settings(&mut self) {
        let profile = self.settings.active_profile();
        let model_options = self.active_model_options();
        let _ = self.ui.open_settings(
            &self.settings,
            api_key_for_profile(&self.api_keys, &profile.id),
            &model_options,
        );
    }
}

pub(super) fn apply_settings_edit(mut settings: AppSettings, edit: SettingsEdit) -> AppSettings {
    settings.hotkey = edit.transcription_hotkey.trim().to_string();
    settings.translation_hotkey = edit.translation_hotkey.trim().to_string();
    settings.recording_mode = parse_recording_mode(&edit.recording_mode);
    settings.silence_threshold_percent = parse_or_keep(
        &edit.silence_threshold_percent,
        settings.silence_threshold_percent,
    );
    settings.silence_timeout_milliseconds = parse_or_keep(
        &edit.silence_timeout_milliseconds,
        settings.silence_timeout_milliseconds,
    );
    settings.max_recording_seconds =
        parse_or_keep(&edit.max_recording_seconds, settings.max_recording_seconds);
    settings.start_with_windows = edit.start_with_windows;
    settings.ui_language = parse_language(&edit.ui_language);
    settings.launch_minimized_to_tray = true;
    settings.show_floating_recording_window = true;
    settings.enable_sounds = edit.enable_sounds;
    settings.restore_clipboard_content = edit.restore_clipboard_content;
    settings.delay_before_paste_milliseconds = parse_or_keep(
        &edit.delay_before_paste_milliseconds,
        settings.delay_before_paste_milliseconds,
    );
    settings.delay_before_clipboard_restore_milliseconds = parse_or_keep(
        &edit.delay_before_clipboard_restore_milliseconds,
        settings.delay_before_clipboard_restore_milliseconds,
    );
    settings.log_level = parse_log_level(&edit.log_level).to_string();

    let active_profile_id = settings.active_api_profile_id.clone();
    if let Some(profile) = settings
        .api_profiles
        .iter_mut()
        .find(|profile| profile.id == active_profile_id)
    {
        profile.name = edit.profile_name.trim().to_string();
        profile.base_url = edit.base_url.trim().to_string();
        profile.model = edit.model.trim().to_string();
        profile.temperature = parse_or_keep(&edit.temperature, profile.temperature);
        profile.request_timeout_seconds = parse_or_keep(
            &edit.request_timeout_seconds,
            profile.request_timeout_seconds,
        );
    }

    settings
}

fn parse_or_keep<T>(value: &str, fallback: T) -> T
where
    T: std::str::FromStr,
{
    value.trim().parse().unwrap_or(fallback)
}

fn parse_recording_mode(value: &str) -> RecordingMode {
    match value.trim() {
        "Hold" | "Удержание" => RecordingMode::Hold,
        "SilenceTimeout" | "Тишина" => RecordingMode::SilenceTimeout,
        _ => RecordingMode::Toggle,
    }
}

fn parse_language(value: &str) -> AppLanguage {
    match value.trim() {
        "English" => AppLanguage::English,
        _ => AppLanguage::Russian,
    }
}

fn parse_log_level(value: &str) -> &'static str {
    match value.trim() {
        "Debug" | "Отладка" => "Debug",
        "Warning" | "Предупреждения" => "Warning",
        "Error" | "Ошибки" => "Error",
        _ => "Information",
    }
}

pub(super) fn settings_saved_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).settings_saved
}

pub(super) fn models_loading_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).models_loading
}

pub(super) fn models_loaded_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).models_loaded
}

pub(super) fn api_profile_added_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).api_profile_added
}

pub(super) fn api_profile_deleted_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).api_profile_deleted
}

pub(super) fn api_profile_selected_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).api_profile_selected
}

pub(super) fn api_profile_required_status(language: AppLanguage) -> &'static str {
    crate::i18n::texts(language).api_profile_required
}

pub(super) fn next_api_profile(existing: &[ApiProfile]) -> ApiProfile {
    let mut index = existing.len() + 1;
    loop {
        let id = format!("profile-{index}");
        if !existing.iter().any(|profile| profile.id == id) {
            return ApiProfile {
                id: id.clone(),
                name: id,
                base_url: "http://localhost:9555".to_string(),
                model: AI2NPU_DEFAULT_MODEL.to_string(),
                language: String::new(),
                temperature: 0.2,
                request_timeout_seconds: 120,
            };
        }
        index += 1;
    }
}

pub(super) fn api_key_for_profile<'a>(
    api_keys: &'a BTreeMap<String, String>,
    profile_id: &str,
) -> &'a str {
    api_keys
        .get(profile_id)
        .or_else(|| api_keys.get("default"))
        .map(String::as_str)
        .unwrap_or("")
}

pub(super) fn profile_id_by_name(settings: &AppSettings, profile_name: &str) -> Option<String> {
    let profile_name = profile_name.trim();
    settings
        .api_profiles
        .iter()
        .find(|profile| profile.name == profile_name)
        .map(|profile| profile.id.clone())
}

pub(super) fn http_client_for_profile(profile: &ApiProfile) -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(profile.request_timeout_seconds))
        .build()?)
}

pub(super) fn profile_requires_http_rebuild(
    old_profile: &ApiProfile,
    new_profile: &ApiProfile,
) -> bool {
    old_profile.request_timeout_seconds != new_profile.request_timeout_seconds
}

pub(super) fn model_for_request(model: &str) -> &str {
    non_empty_str(model).unwrap_or(AI2NPU_DEFAULT_MODEL)
}

pub(super) fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(path)?;

    #[cfg(windows)]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }

    #[cfg(not(windows))]
    {
        let _ = path;
    }

    Ok(())
}
