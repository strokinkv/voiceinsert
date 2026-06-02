#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};

mod background;
mod commands;
mod state;

use crate::api::transcription::AudioRequestKind;
use crate::audio::recorder::{Recorder, RecorderConfig};
use crate::clipboard::ClipboardInserter;
use crate::hotkeys::service::{GlobalHotkeyEvents, HotkeyAction, HotkeyEvents};
use crate::paths::AppPaths;
use crate::settings::AppSettings;
use crate::sounds::{SoundKind, SoundService};
use crate::tray::{RuntimeTray, TrayCommand, TrayState};
use crate::ui::UiController;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

use background::{AudioTask, BackgroundEvent, spawn_load_models_task, spawn_voice_insert_task};
use commands::{api_key_for_profile, http_client_for_profile, model_for_request};
use state::{StateCommand, VoiceInsertState, log_error_message};

/// Starts the single-instance VoiceInsert runtime and enters the Slint event loop.
pub fn run() -> anyhow::Result<()> {
    let _instance = SingleInstanceGuard::acquire("VoiceInsertAppMutex")?;
    if !_instance.acquired {
        return Ok(());
    }

    let runtime = AppRuntime::initialize()?;
    runtime.run()
}

/// Owns application state, UI, tray, hotkeys, audio recording, and background work.
pub struct AppRuntime {
    settings: AppSettings,
    paths: AppPaths,
    api_keys: BTreeMap<String, String>,
    http: reqwest::Client,
    tokio: tokio::runtime::Runtime,
    background_tx: Sender<BackgroundEvent>,
    background_rx: Receiver<BackgroundEvent>,
    state: VoiceInsertState,
    tray: RuntimeTray,
    ui: UiController,
    hotkeys: GlobalHotkeyEvents,
    recorder: Option<Recorder>,
    level_rx: Option<Receiver<LevelSample>>,
    sounds: SoundService,
    clipboard: ClipboardInserter,
    model_options_profile_id: Option<String>,
    model_options: Vec<String>,
}

impl AppRuntime {
    /// Loads settings and secrets, initializes services, and queues model loading.
    pub fn initialize() -> anyhow::Result<Self> {
        let paths = AppPaths::new()?;
        let settings = crate::settings::load_settings(&paths)?;
        let api_keys = crate::secrets::load_api_keys(&paths)?;
        crate::logging::init(&paths, &settings.log_level)?;

        let active_profile = settings.active_profile();
        tracing::info!(
            profile = %active_profile.name,
            base_url = %active_profile.base_url,
            "starting VoiceInsert"
        );

        let state = VoiceInsertState::new(
            settings.recording_mode,
            f32::from(settings.silence_threshold_percent) / 100.0,
            settings.silence_timeout_milliseconds,
            settings.max_recording_seconds.saturating_mul(1000),
        );
        let tray = RuntimeTray::new(settings.ui_language)?;
        let ui = UiController::new();
        let hotkeys = GlobalHotkeyEvents::register(&settings.hotkey, &settings.translation_hotkey)?;
        let clipboard = ClipboardInserter {
            restore_clipboard: settings.restore_clipboard_content,
            delay_before_paste_ms: settings.delay_before_paste_milliseconds,
            delay_before_restore_ms: settings.delay_before_clipboard_restore_milliseconds,
        };
        let sounds = SoundService::new(settings.enable_sounds)?;
        let active_profile = settings.active_profile();
        let http = http_client_for_profile(active_profile)?;
        let tokio = tokio::runtime::Runtime::new()?;
        let (background_tx, background_rx) = std::sync::mpsc::channel();

        let mut runtime = Self {
            settings,
            paths,
            api_keys,
            http,
            tokio,
            background_tx,
            background_rx,
            state,
            tray,
            ui,
            hotkeys,
            recorder: None,
            level_rx: None,
            sounds,
            clipboard,
            model_options_profile_id: None,
            model_options: Vec::new(),
        };
        runtime.queue_model_load_for_active_profile()?;
        Ok(runtime)
    }

    /// Runs the timer-driven desktop event loop until the user exits the app.
    pub fn run(self) -> anyhow::Result<()> {
        let runtime = Rc::new(RefCell::new(self));
        let timer = slint::Timer::default();
        let runtime_for_tick = Rc::clone(&runtime);

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || {
                if let Ok(mut runtime) = runtime_for_tick.try_borrow_mut() {
                    runtime.tick();
                } else {
                    tracing::warn!("runtime tick skipped because the runtime is already borrowed");
                }
            },
        );

        slint::run_event_loop_until_quit()?;
        Ok(())
    }

    fn tick(&mut self) {
        if let Err(error) = self.try_tick() {
            let message = log_error_message(&error);
            tracing::error!(%message, "runtime tick failed");
            self.state.mark_error();
            self.tray.set_state(TrayState::Error);
            self.ui.hide_overlay();
            let _ = self.sounds.play(SoundKind::Error);
        }
    }

    fn try_tick(&mut self) -> anyhow::Result<()> {
        if let Some(command) = self.tray.next_command()
            && self.handle_tray_command(command)?
        {
            slint::quit_event_loop()?;
        }

        if let Some(action) = self.hotkeys.next_event() {
            self.handle_hotkey_action(action);
        }

        for command in self.ui.drain_commands() {
            self.handle_ui_command(command)?;
        }

        self.process_background_events();
        self.process_level_events();
        Ok(())
    }

    fn handle_tray_command(&mut self, command: TrayCommand) -> anyhow::Result<bool> {
        match command {
            TrayCommand::Settings => {
                tracing::info!("settings command received");
                let profile = self.settings.active_profile();
                let model_options = self.active_model_options();
                self.ui.open_settings(
                    &self.settings,
                    api_key_for_profile(&self.api_keys, &profile.id),
                    &model_options,
                )?;
                Ok(false)
            }
            TrayCommand::Exit => {
                tracing::info!("exit command received");
                Ok(true)
            }
        }
    }

    fn handle_hotkey_action(&mut self, action: HotkeyAction) {
        let command = match action {
            HotkeyAction::TranscribePressed => {
                self.state.hotkey_pressed(AudioRequestKind::Transcription)
            }
            HotkeyAction::TranslatePressed => {
                self.state.hotkey_pressed(AudioRequestKind::Translation)
            }
            HotkeyAction::CancelPressed => self.state.cancel_recording(),
            HotkeyAction::Released => self.state.hotkey_released(),
        };

        if let Err(error) = self.apply_state_command(command) {
            let message = log_error_message(&error);
            tracing::error!(%message, "failed to handle hotkey action");
            self.state.mark_error();
            self.tray.set_state(TrayState::Error);
            self.ui.hide_overlay();
            let _ = self.sounds.play(SoundKind::Error);
        }
    }

    fn apply_state_command(&mut self, command: StateCommand) -> anyhow::Result<()> {
        match command {
            StateCommand::None => {}
            StateCommand::StartRecording(kind) => {
                tracing::info!(?kind, "recording started");
                self.start_recording()?;
                self.ui.show_recording_overlay()?;
                self.tray.set_state(TrayState::Recording);
                let _ = self.sounds.play(SoundKind::Start);
            }
            StateCommand::StopAndTranscribe(kind) => {
                tracing::info!(?kind, "recording stopped");
                self.tray.set_state(TrayState::Transcribing);
                self.ui.set_overlay_status("Transcribing")?;
                let _ = self.sounds.play(SoundKind::Stop);
                self.stop_recording_and_insert(kind)?;
            }
            StateCommand::CancelRecording => {
                tracing::info!("recording cancelled");
                self.stop_recording_without_insert();
                self.tray.set_state(TrayState::Idle);
                self.ui.hide_overlay();
                let _ = self.sounds.play(SoundKind::Stop);
            }
        }

        Ok(())
    }

    fn start_recording(&mut self) -> anyhow::Result<()> {
        let max_buffer_samples = self
            .settings
            .max_recording_seconds
            .saturating_mul(crate::audio::recorder::TARGET_SAMPLE_RATE as u64)
            .try_into()
            .unwrap_or(usize::MAX);
        let mut recorder = Recorder::new(RecorderConfig {
            device_index: None,
            max_buffer_samples,
            ..RecorderConfig::default()
        });
        let (level_tx, level_rx) = std::sync::mpsc::channel();

        recorder.start(move |level, delta_ms| {
            let _ = send_level(&level_tx, level, delta_ms);
        })?;

        self.recorder = Some(recorder);
        self.level_rx = Some(level_rx);
        Ok(())
    }

    fn stop_recording_and_insert(&mut self, kind: AudioRequestKind) -> anyhow::Result<()> {
        let wav_bytes = match self.recorder.as_mut() {
            Some(recorder) => recorder.stop()?,
            None => anyhow::bail!("recording stop requested but recorder is not active"),
        };
        self.recorder = None;
        self.level_rx = None;

        let profile = self.settings.active_profile();
        spawn_voice_insert_task(
            self.tokio.handle().clone(),
            self.http.clone(),
            self.clipboard,
            AudioTask {
                base_url: profile.base_url.clone(),
                api_key: api_key_for_profile(&self.api_keys, &profile.id).to_string(),
                model: model_for_request(&profile.model).to_string(),
                language: None,
                temperature: profile.temperature,
                wav_bytes,
                kind,
                target_window: active_window(),
            },
            self.background_tx.clone(),
        );
        Ok(())
    }

    fn stop_recording_without_insert(&mut self) {
        if let Some(mut recorder) = self.recorder.take() {
            let _ = recorder.stop();
        }
        self.level_rx = None;
    }

    fn process_background_events(&mut self) {
        loop {
            match self.background_rx.try_recv() {
                Ok(BackgroundEvent::Error(message)) => self.ui.set_status(message),
                Ok(BackgroundEvent::ModelsLoaded { profile_id, models }) => {
                    if profile_id == self.settings.active_profile().id {
                        self.model_options_profile_id = Some(profile_id);
                        self.model_options = models;
                        let current_model =
                            model_for_request(&self.settings.active_profile().model);
                        self.ui
                            .set_model_options(current_model, &self.model_options);
                    }
                }
                Ok(BackgroundEvent::VoiceInsertionStarted) => {
                    self.state.mark_inserting();
                    let _ = self.ui.set_overlay_status("Inserting");
                }
                Ok(BackgroundEvent::VoiceInsertionComplete(Ok(()))) => {
                    self.state.mark_idle();
                    self.tray.set_state(TrayState::Idle);
                    self.ui.hide_overlay();
                }
                Ok(BackgroundEvent::VoiceInsertionComplete(Err(message))) => {
                    tracing::error!(%message, "voice insertion task failed");
                    self.state.mark_error();
                    self.tray.set_state(TrayState::Error);
                    self.ui.hide_overlay();
                    let _ = self.sounds.play(SoundKind::Error);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn process_level_events(&mut self) {
        let mut levels = Vec::new();
        if let Some(level_rx) = &self.level_rx {
            loop {
                match level_rx.try_recv() {
                    Ok(level) => levels.push(level),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }

        for sample in levels {
            self.ui
                .set_overlay_level(sample.level, self.state.silence_threshold);
            let command = self.state.level_changed(sample.level, sample.delta_ms);
            if let Err(error) = self.apply_state_command(command) {
                let message = log_error_message(&error);
                tracing::error!(%message, "failed to process audio level");
                self.state.mark_error();
                self.tray.set_state(TrayState::Error);
                self.ui.hide_overlay();
                let _ = self.sounds.play(SoundKind::Error);
                break;
            }
        }
    }

    fn active_model_options(&self) -> Vec<String> {
        if self
            .model_options_profile_id
            .as_deref()
            .is_some_and(|profile_id| profile_id == self.settings.active_profile().id)
        {
            self.model_options.clone()
        } else {
            Vec::new()
        }
    }

    fn queue_model_load_for_active_profile(&mut self) -> anyhow::Result<()> {
        let profile = self.settings.active_profile();
        let profile_id = profile.id.clone();
        let base_url = profile.base_url.clone();
        let api_key = api_key_for_profile(&self.api_keys, &profile_id).to_string();
        let current_model = model_for_request(&profile.model).to_string();
        self.model_options_profile_id = Some(profile_id.clone());
        self.model_options.clear();
        self.ui
            .set_model_options(&current_model, &self.model_options);
        spawn_load_models_task(
            self.tokio.handle().clone(),
            self.http.clone(),
            profile_id,
            base_url,
            api_key,
            self.background_tx.clone(),
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct LevelSample {
    level: f32,
    delta_ms: u64,
}

fn send_level(
    level_tx: &Sender<LevelSample>,
    level: f32,
    delta_ms: u64,
) -> Result<(), std::sync::mpsc::SendError<LevelSample>> {
    level_tx.send(LevelSample { level, delta_ms })
}

pub(super) fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(windows)]
fn active_window() -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        None
    } else {
        Some(hwnd.0 as isize)
    }
}

#[cfg(not(windows))]
fn active_window() -> Option<isize> {
    None
}

#[derive(Debug)]
pub struct SingleInstanceGuard {
    acquired: bool,
    #[cfg(windows)]
    handle: HANDLE,
}

impl SingleInstanceGuard {
    /// Acquires the named application mutex and reports whether this process owns it.
    pub fn acquire(name: &str) -> anyhow::Result<Self> {
        acquire_single_instance(name)
    }

    /// Returns true when this process should continue startup.
    pub fn acquired(&self) -> bool {
        self.acquired
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
fn acquire_single_instance(name: &str) -> anyhow::Result<SingleInstanceGuard> {
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::PCWSTR;

    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(None, false, PCWSTR(wide_name.as_ptr()))? };
    let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;

    Ok(SingleInstanceGuard {
        acquired: !already_exists,
        handle,
    })
}

#[cfg(not(windows))]
fn acquire_single_instance(_name: &str) -> anyhow::Result<SingleInstanceGuard> {
    Ok(SingleInstanceGuard { acquired: true })
}

#[cfg(test)]
mod tests {
    use super::SingleInstanceGuard;
    use super::commands::{
        api_key_for_profile, apply_settings_edit, model_for_request, profile_id_by_name,
        profile_requires_http_rebuild,
    };
    use crate::settings::{ApiProfile, RecordingMode};
    use crate::ui::SettingsEdit;
    use std::collections::BTreeMap;

    #[test]
    fn single_instance_guard_reports_acquired_on_non_conflicting_name() {
        let name = format!("VoiceInsertTestMutex{}", std::process::id());
        let guard = SingleInstanceGuard::acquire(&name).unwrap();

        assert!(guard.acquired());
    }

    #[test]
    fn api_key_prefers_active_profile_then_default() {
        let mut keys = BTreeMap::new();
        keys.insert("default".to_string(), "default-key".to_string());
        keys.insert("ai2npu".to_string(), "profile-key".to_string());

        assert_eq!(api_key_for_profile(&keys, "ai2npu"), "profile-key");
        assert_eq!(api_key_for_profile(&keys, "missing"), "default-key");
    }

    #[test]
    fn profile_id_lookup_uses_visible_profile_name() {
        let settings = crate::settings::AppSettings::default().normalized();

        assert_eq!(
            profile_id_by_name(&settings, "groq"),
            Some("groq".to_string())
        );
        assert_eq!(profile_id_by_name(&settings, "missing"), None);
    }

    #[test]
    fn empty_model_uses_transcription_fallback() {
        assert_eq!(model_for_request(""), "openai/whisper-large-v3-turbo");
        assert_eq!(model_for_request(" custom-model "), "custom-model");
    }

    #[test]
    fn apply_settings_edit_updates_active_profile_and_hotkeys() {
        let mut settings = crate::settings::AppSettings::default().normalized();
        settings.api_profiles[0].language = "en".to_string();
        let settings = apply_settings_edit(
            settings,
            SettingsEdit {
                profile_name: "ai2npu".to_string(),
                base_url: " http://localhost:9555 ".to_string(),
                api_key: String::new(),
                model: " whisper-large-v3 ".to_string(),
                temperature: "0.4".to_string(),
                request_timeout_seconds: "45".to_string(),
                transcription_hotkey: " Ctrl+Space ".to_string(),
                translation_hotkey: " Alt+Y ".to_string(),
                recording_mode: "Hold".to_string(),
                silence_threshold_percent: "8".to_string(),
                silence_timeout_milliseconds: "900".to_string(),
                max_recording_seconds: "30".to_string(),
                start_with_windows: true,
                ui_language: "English".to_string(),
                enable_sounds: false,
                restore_clipboard_content: false,
                delay_before_paste_milliseconds: "120".to_string(),
                delay_before_clipboard_restore_milliseconds: "500".to_string(),
                log_level: "Debug".to_string(),
            },
        )
        .normalized();

        assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
        assert_eq!(settings.active_profile().model, "whisper-large-v3");
        assert_eq!(settings.active_profile().language, "en");
        assert_eq!(settings.active_profile().temperature, 0.4);
        assert_eq!(settings.active_profile().request_timeout_seconds, 45);
        assert_eq!(settings.hotkey, "Ctrl+Space");
        assert_eq!(settings.translation_hotkey, "Alt+Y");
        assert_eq!(settings.recording_mode, RecordingMode::Hold);
        assert_eq!(settings.silence_threshold_percent, 8);
        assert_eq!(settings.silence_timeout_milliseconds, 900);
        assert_eq!(settings.max_recording_seconds, 30);
        assert!(settings.start_with_windows);
        assert!(settings.launch_minimized_to_tray);
        assert!(settings.show_floating_recording_window);
        assert!(!settings.enable_sounds);
        assert!(!settings.restore_clipboard_content);
        assert_eq!(settings.delay_before_paste_milliseconds, 120);
        assert_eq!(settings.delay_before_clipboard_restore_milliseconds, 500);
        assert_eq!(settings.log_level, "Debug");
    }

    #[test]
    fn profile_timeout_change_requires_http_rebuild() {
        let old_profile = ApiProfile {
            request_timeout_seconds: 30,
            ..ApiProfile::default()
        };
        let new_profile = ApiProfile {
            request_timeout_seconds: 120,
            ..old_profile.clone()
        };

        assert!(profile_requires_http_rebuild(&old_profile, &new_profile));
    }
}
