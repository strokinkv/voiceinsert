slint::include_modules!();

use crate::overlay::{overlay_position, primary_work_area};
use crate::settings::{AI2NPU_DEFAULT_MODEL, AppLanguage, AppSettings, RecordingMode};
use slint::{
    CloseRequestResponse, ComponentHandle, ModelRc, PhysicalPosition, SharedString, VecModel,
};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    SettingsChanged,
    AddApiProfile,
    DeleteApiProfile,
    SelectApiProfile(String),
    OpenLogsFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsEdit {
    pub profile_name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: String,
    pub request_timeout_seconds: String,
    pub transcription_hotkey: String,
    pub translation_hotkey: String,
    pub recording_mode: String,
    pub silence_threshold_percent: String,
    pub silence_timeout_milliseconds: String,
    pub max_recording_seconds: String,
    pub start_with_windows: bool,
    pub ui_language: String,
    pub enable_sounds: bool,
    pub restore_clipboard_content: bool,
    pub delay_before_paste_milliseconds: String,
    pub delay_before_clipboard_restore_milliseconds: String,
    pub log_level: String,
}

pub struct UiController {
    settings_window: Option<SettingsWindow>,
    recording_overlay: Option<RecordingOverlay>,
    command_tx: Sender<UiCommand>,
    command_rx: Receiver<UiCommand>,
}

impl UiController {
    pub fn new() -> Self {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        Self {
            settings_window: None,
            recording_overlay: None,
            command_tx,
            command_rx,
        }
    }

    pub fn open_settings(
        &mut self,
        settings: &AppSettings,
        api_key: &str,
        model_options: &[String],
    ) -> anyhow::Result<()> {
        let window = match &self.settings_window {
            Some(window) => window.clone_strong(),
            None => {
                let window = SettingsWindow::new()?;
                wire_settings_callbacks(&window, self.command_tx.clone());
                self.settings_window = Some(window.clone_strong());
                window
            }
        };

        let profile = settings.active_profile();
        let model_name = model_label(&profile.model).to_string();
        let profile_options = api_profile_options_for_display(settings);
        window.set_api_profile_options(shared_string_model(&profile_options));
        window.set_active_profile_name(SharedString::from(profile.name.as_str()));
        window.set_profile_name(SharedString::from(profile.name.as_str()));
        window.set_api_base_url(SharedString::from(profile.base_url.as_str()));
        window.set_api_key(SharedString::from(api_key));
        window.set_model_name(SharedString::from(model_name.as_str()));
        window.set_model_options(shared_string_model(&model_options_for_display(
            &model_name,
            model_options,
        )));
        window.set_temperature(SharedString::from(format!("{:.1}", profile.temperature)));
        window.set_request_timeout_seconds(SharedString::from(
            profile.request_timeout_seconds.to_string(),
        ));
        window.set_transcription_hotkey(SharedString::from(settings.hotkey.as_str()));
        window.set_translation_hotkey(SharedString::from(settings.translation_hotkey.as_str()));
        window.set_recording_mode(SharedString::from(recording_mode_display_label(
            settings.recording_mode,
            settings.ui_language,
        )));
        window.set_silence_threshold_percent(SharedString::from(
            settings.silence_threshold_percent.to_string(),
        ));
        window.set_silence_timeout_milliseconds(SharedString::from(
            settings.silence_timeout_milliseconds.to_string(),
        ));
        window.set_max_recording_seconds(SharedString::from(
            settings.max_recording_seconds.to_string(),
        ));
        window.set_start_with_windows(settings.start_with_windows);
        window.set_ui_language(SharedString::from(language_label(settings.ui_language)));
        window.set_enable_sounds(settings.enable_sounds);
        window.set_restore_clipboard_content(settings.restore_clipboard_content);
        window.set_delay_before_paste_milliseconds(SharedString::from(
            settings.delay_before_paste_milliseconds.to_string(),
        ));
        window.set_delay_before_clipboard_restore_milliseconds(SharedString::from(
            settings
                .delay_before_clipboard_restore_milliseconds
                .to_string(),
        ));
        window.set_log_level(SharedString::from(log_level_display_label(
            &settings.log_level,
            settings.ui_language,
        )));
        window.set_status_text(SharedString::from(""));
        window.show()?;
        Ok(())
    }

    pub fn show_recording_overlay(&mut self) -> anyhow::Result<()> {
        let overlay = self.overlay()?;
        overlay.set_status_text(SharedString::from("Recording"));
        overlay.set_level(0.0);
        overlay.set_is_silent(false);
        position_overlay_near_clock(&overlay);
        overlay.show()?;
        Ok(())
    }

    pub fn set_overlay_level(&mut self, level: f32, silence_threshold: f32) {
        if let Some(overlay) = &self.recording_overlay {
            let level = level.clamp(0.0, 1.0);
            overlay.set_level(level);
            overlay.set_is_silent(level <= silence_threshold.clamp(0.0, 1.0));
        }
    }

    pub fn set_overlay_status(&mut self, status: &str) -> anyhow::Result<()> {
        let overlay = self.overlay()?;
        overlay.set_status_text(SharedString::from(status));
        position_overlay_near_clock(&overlay);
        overlay.show()?;
        Ok(())
    }

    pub fn hide_overlay(&self) {
        if let Some(overlay) = &self.recording_overlay {
            let _ = overlay.hide();
        }
    }

    pub fn set_status(&self, message: impl Into<SharedString>) {
        if let Some(window) = &self.settings_window {
            window.set_status_text(message.into());
        }
    }

    pub fn set_profile_metadata(&self, settings: &AppSettings) {
        if let Some(window) = &self.settings_window {
            let profile = settings.active_profile();
            window.set_api_profile_options(shared_string_model(&api_profile_options_for_display(
                settings,
            )));
            window.set_active_profile_name(SharedString::from(profile.name.as_str()));
        }
    }

    pub fn set_model_options(&self, current_model: &str, model_options: &[String]) {
        if let Some(window) = &self.settings_window {
            let visible_model = window.get_model_name().to_string();
            let current_model = if visible_model.trim().is_empty() {
                current_model
            } else {
                visible_model.trim()
            };
            window.set_model_options(shared_string_model(&model_options_for_display(
                current_model,
                model_options,
            )));
        }
    }

    pub fn drain_commands(&self) -> Vec<UiCommand> {
        let mut commands = Vec::new();
        loop {
            match self.command_rx.try_recv() {
                Ok(command) => commands.push(command),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        commands
    }

    pub fn settings_edit(&self) -> Option<SettingsEdit> {
        let window = self.settings_window.as_ref()?;
        Some(SettingsEdit {
            profile_name: window.get_profile_name().to_string(),
            base_url: window.get_api_base_url().to_string(),
            api_key: window.get_api_key().to_string(),
            model: window.get_model_name().to_string(),
            temperature: window.get_temperature().to_string(),
            request_timeout_seconds: window.get_request_timeout_seconds().to_string(),
            transcription_hotkey: window.get_transcription_hotkey().to_string(),
            translation_hotkey: window.get_translation_hotkey().to_string(),
            recording_mode: window.get_recording_mode().to_string(),
            silence_threshold_percent: window.get_silence_threshold_percent().to_string(),
            silence_timeout_milliseconds: window.get_silence_timeout_milliseconds().to_string(),
            max_recording_seconds: window.get_max_recording_seconds().to_string(),
            start_with_windows: window.get_start_with_windows(),
            ui_language: window.get_ui_language().to_string(),
            enable_sounds: window.get_enable_sounds(),
            restore_clipboard_content: window.get_restore_clipboard_content(),
            delay_before_paste_milliseconds: window
                .get_delay_before_paste_milliseconds()
                .to_string(),
            delay_before_clipboard_restore_milliseconds: window
                .get_delay_before_clipboard_restore_milliseconds()
                .to_string(),
            log_level: window.get_log_level().to_string(),
        })
    }

    fn overlay(&mut self) -> anyhow::Result<RecordingOverlay> {
        match &self.recording_overlay {
            Some(overlay) => Ok(overlay.clone_strong()),
            None => {
                let overlay = RecordingOverlay::new()?;
                self.recording_overlay = Some(overlay.clone_strong());
                Ok(overlay)
            }
        }
    }
}

fn position_overlay_near_clock(overlay: &RecordingOverlay) {
    let width = overlay.window().size().width as i32;
    let height = overlay.window().size().height as i32;
    let work_area = primary_work_area();
    let (x, y) = overlay_position(work_area, (width, height), 24);
    overlay.window().set_position(PhysicalPosition::new(x, y));
}

fn model_label(model: &str) -> &str {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        AI2NPU_DEFAULT_MODEL
    } else {
        trimmed
    }
}

fn model_options_for_display(current_model: &str, options: &[String]) -> Vec<String> {
    let mut values = Vec::new();
    let current_model = model_label(current_model).to_string();
    if !current_model.trim().is_empty() {
        values.push(current_model.clone());
    }

    for option in options {
        let trimmed = option.trim();
        if !trimmed.is_empty() && !values.iter().any(|value| value == trimmed) {
            values.push(trimmed.to_string());
        }
    }

    values
}

fn api_profile_options_for_display(settings: &AppSettings) -> Vec<String> {
    let active_profile = settings.active_profile();
    let mut values = vec![active_profile.name.clone()];
    for profile in &settings.api_profiles {
        if !profile.name.trim().is_empty() && !values.iter().any(|value| value == &profile.name) {
            values.push(profile.name.clone());
        }
    }
    values
}

fn shared_string_model(values: &[String]) -> ModelRc<SharedString> {
    let rows = values
        .iter()
        .map(|value| SharedString::from(value.as_str()))
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

fn recording_mode_display_label(mode: RecordingMode, language: AppLanguage) -> &'static str {
    match (language, mode) {
        (AppLanguage::Russian, RecordingMode::Toggle) => "Переключатель",
        (AppLanguage::Russian, RecordingMode::Hold) => "Удержание",
        (AppLanguage::Russian, RecordingMode::SilenceTimeout) => "Тишина",
        (_, RecordingMode::Toggle) => "Toggle",
        (_, RecordingMode::Hold) => "Hold",
        (_, RecordingMode::SilenceTimeout) => "SilenceTimeout",
    }
}

fn log_level_display_label(level: &str, language: AppLanguage) -> &'static str {
    match (language, level) {
        (AppLanguage::Russian, "Debug") => "Отладка",
        (AppLanguage::Russian, "Warning") => "Предупреждения",
        (AppLanguage::Russian, "Error") => "Ошибки",
        (AppLanguage::Russian, _) => "Инфо",
        (_, "Debug") => "Debug",
        (_, "Warning") => "Warning",
        (_, "Error") => "Error",
        _ => "Information",
    }
}

fn language_label(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::Russian => "Russian",
        AppLanguage::English => "English",
    }
}

impl Default for UiController {
    fn default() -> Self {
        Self::new()
    }
}

fn wire_settings_callbacks(window: &SettingsWindow, command_tx: Sender<UiCommand>) {
    let weak_window = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(window) = weak_window.upgrade() {
            let _ = window.hide();
        }
        CloseRequestResponse::KeepWindowShown
    });

    let tx = command_tx.clone();
    window.on_settings_changed(move || {
        let _ = tx.send(UiCommand::SettingsChanged);
    });

    let tx = command_tx.clone();
    window.on_add_api_profile(move || {
        let _ = tx.send(UiCommand::AddApiProfile);
    });

    let tx = command_tx.clone();
    window.on_delete_api_profile(move || {
        let _ = tx.send(UiCommand::DeleteApiProfile);
    });

    let tx = command_tx.clone();
    window.on_select_api_profile(move |profile_name| {
        let _ = tx.send(UiCommand::SelectApiProfile(profile_name.to_string()));
    });

    let tx = command_tx.clone();
    window.on_open_logs_folder(move || {
        let _ = tx.send(UiCommand::OpenLogsFolder);
    });
}

#[cfg(test)]
mod tests {
    use super::{api_profile_options_for_display, model_label, model_options_for_display};

    #[test]
    fn empty_model_label_uses_default_transcription_model() {
        assert_eq!(model_label(""), "openai/whisper-large-v3-turbo");
        assert_eq!(model_label(" custom "), "custom");
    }

    #[test]
    fn model_options_include_current_model_first_and_deduplicate() {
        let options = vec![
            "openai/whisper-large-v3-turbo".to_string(),
            "custom".to_string(),
            " custom ".to_string(),
        ];

        assert_eq!(
            model_options_for_display("custom", &options),
            vec![
                "custom".to_string(),
                "openai/whisper-large-v3-turbo".to_string()
            ]
        );
    }

    #[test]
    fn api_profile_options_include_active_profile_first() {
        let mut settings = crate::settings::AppSettings::default().normalized();
        settings.active_api_profile_id = "groq".to_string();

        assert_eq!(
            api_profile_options_for_display(&settings),
            vec!["groq".to_string(), "ai2npu".to_string()]
        );
    }
}
