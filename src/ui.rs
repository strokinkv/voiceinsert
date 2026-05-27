slint::include_modules!();

use crate::settings::AppSettings;
use slint::{ComponentHandle, SharedString};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    Save,
    LoadModels,
    TestApiConnection,
    OpenLogsFolder,
    ClearLogs,
}

pub struct UiController {
    settings_window: Option<SettingsWindow>,
    command_tx: Sender<UiCommand>,
    command_rx: Receiver<UiCommand>,
}

impl UiController {
    pub fn new() -> Self {
        let (command_tx, command_rx) = std::sync::mpsc::channel();
        Self {
            settings_window: None,
            command_tx,
            command_rx,
        }
    }

    pub fn open_settings(&mut self, settings: &AppSettings) -> anyhow::Result<()> {
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
        window.set_profile_line(SharedString::from(format!("Profile: {}", profile.name)));
        window.set_api_line(SharedString::from(format!("API: {}", profile.base_url)));
        window.set_model_line(SharedString::from(format!(
            "Model: {}",
            model_label(&profile.model)
        )));
        window.set_transcription_hotkey_line(SharedString::from(format!(
            "Transcription: {}",
            settings.hotkey
        )));
        window.set_translation_hotkey_line(SharedString::from(format!(
            "Translation: {}",
            settings.translation_hotkey
        )));
        window.set_recording_mode_line(SharedString::from(format!(
            "Recording: {:?}",
            settings.recording_mode
        )));
        window.set_status_text(SharedString::from(format!(
            "Profile: {} ({})",
            profile.name, profile.base_url
        )));
        window.show()?;
        Ok(())
    }

    pub fn set_status(&self, message: impl Into<SharedString>) {
        if let Some(window) = &self.settings_window {
            window.set_status_text(message.into());
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
}

fn model_label(model: &str) -> &str {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        "whisper-large-v3"
    } else {
        trimmed
    }
}

impl Default for UiController {
    fn default() -> Self {
        Self::new()
    }
}

fn wire_settings_callbacks(window: &SettingsWindow, command_tx: Sender<UiCommand>) {
    let weak = window.as_weak();
    window.on_close(move || {
        if let Some(window) = weak.upgrade() {
            let _ = window.hide();
        }
    });

    let tx = command_tx.clone();
    window.on_save(move || {
        let _ = tx.send(UiCommand::Save);
    });

    let tx = command_tx.clone();
    window.on_load_models(move || {
        let _ = tx.send(UiCommand::LoadModels);
    });

    let tx = command_tx.clone();
    window.on_test_api_connection(move || {
        let _ = tx.send(UiCommand::TestApiConnection);
    });

    let tx = command_tx.clone();
    window.on_open_logs_folder(move || {
        let _ = tx.send(UiCommand::OpenLogsFolder);
    });

    window.on_clear_logs(move || {
        let _ = command_tx.send(UiCommand::ClearLogs);
    });
}

#[cfg(test)]
mod tests {
    use super::model_label;

    #[test]
    fn empty_model_label_uses_default_transcription_model() {
        assert_eq!(model_label(""), "whisper-large-v3");
        assert_eq!(model_label(" custom "), "custom");
    }
}
