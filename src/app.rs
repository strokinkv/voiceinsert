#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};

use crate::api::transcription::AudioRequestKind;
use crate::api::transcription::{SendAudioRequest, send_audio};
use crate::api::{health::check_profile, models::load_models};
use crate::audio::levels::should_stop_on_silence;
use crate::audio::recorder::{Recorder, RecorderConfig};
use crate::clipboard::ClipboardInserter;
use crate::hotkeys::service::{GlobalHotkeyEvents, HotkeyAction, HotkeyEvents};
use crate::logging::LastErrorState;
use crate::paths::AppPaths;
use crate::settings::{AI2NPU_DEFAULT_MODEL, AppSettings, RecordingMode};
use crate::sounds::{SoundKind, SoundService};
use crate::tray::{RuntimeTray, TrayCommand, TrayState};
use crate::ui::{SettingsEdit, UiCommand, UiController};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

pub fn run() -> anyhow::Result<()> {
    let _instance = SingleInstanceGuard::acquire("VoiceInsertAppMutex")?;
    if !_instance.acquired {
        return Ok(());
    }

    let runtime = AppRuntime::initialize()?;
    runtime.run()
}

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
    last_error: LastErrorState,
}

impl AppRuntime {
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
        let sounds = SoundService {
            enabled: settings.enable_sounds,
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(settings.request_timeout_seconds))
            .build()?;
        let tokio = tokio::runtime::Runtime::new()?;
        let (background_tx, background_rx) = std::sync::mpsc::channel();

        Ok(Self {
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
            last_error: LastErrorState::default(),
        })
    }

    pub fn run(self) -> anyhow::Result<()> {
        let runtime = Rc::new(RefCell::new(self));
        let timer = slint::Timer::default();
        let runtime_for_tick = Rc::clone(&runtime);

        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(16),
            move || runtime_for_tick.borrow_mut().tick(),
        );

        slint::run_event_loop_until_quit()?;
        Ok(())
    }

    fn tick(&mut self) {
        if let Err(error) = self.try_tick() {
            let message = error.to_string();
            self.last_error.set(message.clone());
            tracing::error!(%message, "runtime tick failed");
            self.state.mark_error();
            self.tray.set_state(TrayState::Error);
            let _ = self.ui.set_overlay_status("Error");
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
                self.ui.open_settings(&self.settings)?;
                Ok(false)
            }
            TrayCommand::Exit => {
                tracing::info!("exit command received");
                Ok(true)
            }
        }
    }

    fn handle_ui_command(&mut self, command: UiCommand) -> anyhow::Result<()> {
        match command {
            UiCommand::Save => {
                if let Some(edit) = self.ui.settings_edit() {
                    crate::hotkeys::matcher::validate_pair(
                        &edit.transcription_hotkey,
                        &edit.translation_hotkey,
                    )?;
                    crate::api::endpoints::endpoint(&edit.base_url, "/v1/models")?;
                    let old_hotkey = self.settings.hotkey.clone();
                    let old_translation_hotkey = self.settings.translation_hotkey.clone();
                    self.settings = apply_settings_edit(self.settings.clone(), edit).normalized();
                    if self.settings.hotkey != old_hotkey
                        || self.settings.translation_hotkey != old_translation_hotkey
                    {
                        self.hotkeys = GlobalHotkeyEvents::register(
                            &self.settings.hotkey,
                            &self.settings.translation_hotkey,
                        )?;
                    }
                }
                crate::settings::save_settings(&self.paths, &self.settings)?;
                self.ui.set_status("Settings saved.");
            }
            UiCommand::LoadModels => {
                let profile = self.settings.active_profile();
                self.ui.set_status("Loading models...");
                spawn_load_models_task(
                    self.tokio.handle().clone(),
                    self.http.clone(),
                    profile.base_url.clone(),
                    api_key_for_profile(&self.api_keys, &profile.id).to_string(),
                    self.background_tx.clone(),
                )?;
            }
            UiCommand::TestApiConnection => {
                let profile = self.settings.active_profile();
                self.ui.set_status("Checking API connection...");
                spawn_health_check_task(
                    self.tokio.handle().clone(),
                    self.http.clone(),
                    profile.clone(),
                    api_key_for_profile(&self.api_keys, &profile.id).to_string(),
                    self.background_tx.clone(),
                )?;
            }
            UiCommand::OpenLogsFolder => {
                open_folder(&self.paths.logs_dir())?;
            }
            UiCommand::ClearLogs => {
                clear_logs(&self.paths)?;
                self.ui.set_status("Logs cleared.");
            }
        }

        Ok(())
    }

    fn handle_hotkey_action(&mut self, action: HotkeyAction) {
        let command = match action {
            HotkeyAction::TranscribePressed => {
                self.state.hotkey_pressed(AudioRequestKind::Transcription)
            }
            HotkeyAction::TranslatePressed => {
                self.state.hotkey_pressed(AudioRequestKind::Translation)
            }
            HotkeyAction::Released => self.state.hotkey_released(),
        };

        if let Err(error) = self.apply_state_command(command) {
            let message = error.to_string();
            self.last_error.set(message.clone());
            tracing::error!(%message, "failed to handle hotkey action");
            self.state.mark_error();
            self.tray.set_state(TrayState::Error);
            let _ = self.ui.set_overlay_status("Error");
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
            max_buffer_samples,
            ..RecorderConfig::default()
        });
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        let mut last_level_at = Instant::now();

        recorder.start(move |level| {
            let now = Instant::now();
            let delta_ms = now.duration_since(last_level_at).as_millis() as u64;
            last_level_at = now;
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
                language: non_empty_str(&profile.language).map(str::to_string),
                temperature: profile.temperature,
                wav_bytes,
                kind,
                target_window: active_window(),
            },
            self.background_tx.clone(),
        )?;
        Ok(())
    }

    fn process_background_events(&mut self) {
        loop {
            match self.background_rx.try_recv() {
                Ok(BackgroundEvent::Status(message)) => self.ui.set_status(message),
                Ok(BackgroundEvent::Error(message)) => {
                    self.last_error.set(message.clone());
                    self.ui.set_status(message);
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
                    self.last_error.set(message.clone());
                    tracing::error!(%message, "voice insertion task failed");
                    self.state.mark_error();
                    self.tray.set_state(TrayState::Error);
                    let _ = self.ui.set_overlay_status("Error");
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
                let message = error.to_string();
                self.last_error.set(message.clone());
                tracing::error!(%message, "failed to process audio level");
                self.state.mark_error();
                self.tray.set_state(TrayState::Error);
                let _ = self.ui.set_overlay_status("Error");
                let _ = self.sounds.play(SoundKind::Error);
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LevelSample {
    level: f32,
    delta_ms: u64,
}

#[derive(Debug)]
enum BackgroundEvent {
    Status(String),
    Error(String),
    VoiceInsertionStarted,
    VoiceInsertionComplete(Result<(), String>),
}

struct AudioTask {
    base_url: String,
    api_key: String,
    model: String,
    language: Option<String>,
    temperature: f64,
    wav_bytes: Vec<u8>,
    kind: AudioRequestKind,
    target_window: Option<isize>,
}

fn spawn_voice_insert_task(
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    clipboard: ClipboardInserter,
    task: AudioTask,
    events: Sender<BackgroundEvent>,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("voiceinsert-api".to_string())
        .spawn(move || {
            let AudioTask {
                base_url,
                api_key,
                model,
                language,
                temperature,
                wav_bytes,
                kind,
                target_window,
            } = task;
            let completion = handle.block_on(async {
                let text = send_audio(
                    &http,
                    SendAudioRequest {
                        base_url: &base_url,
                        api_key: &api_key,
                        model: &model,
                        language: language.as_deref(),
                        temperature: Some(temperature),
                        wav_bytes,
                        kind,
                    },
                )
                .await?;

                let _ = events.send(BackgroundEvent::VoiceInsertionStarted);
                clipboard.insert_text(&text, target_window).await
            });

            let _ = events.send(BackgroundEvent::VoiceInsertionComplete(
                completion.map_err(|error| error.to_string()),
            ));
        })?;

    Ok(())
}

fn spawn_load_models_task(
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    events: Sender<BackgroundEvent>,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("voiceinsert-load-models".to_string())
        .spawn(move || {
            let result = handle.block_on(load_models(&http, &base_url, &api_key));
            match result {
                Ok(models) => {
                    let _ = events.send(BackgroundEvent::Status(format!(
                        "Loaded models: {}.",
                        models.len()
                    )));
                }
                Err(error) => {
                    let _ = events.send(BackgroundEvent::Error(error.to_string()));
                }
            }
        })?;

    Ok(())
}

fn spawn_health_check_task(
    handle: tokio::runtime::Handle,
    http: reqwest::Client,
    profile: crate::settings::ApiProfile,
    api_key: String,
    events: Sender<BackgroundEvent>,
) -> anyhow::Result<()> {
    std::thread::Builder::new()
        .name("voiceinsert-health".to_string())
        .spawn(move || {
            let result = handle.block_on(check_profile(&http, &profile, &api_key));
            let event = if result.is_healthy {
                BackgroundEvent::Status(result.message)
            } else {
                BackgroundEvent::Error(result.message)
            };
            let _ = events.send(event);
        })?;

    Ok(())
}

fn send_level(
    level_tx: &Sender<LevelSample>,
    level: f32,
    delta_ms: u64,
) -> Result<(), std::sync::mpsc::SendError<LevelSample>> {
    level_tx.send(LevelSample { level, delta_ms })
}

fn apply_settings_edit(mut settings: AppSettings, edit: SettingsEdit) -> AppSettings {
    settings.hotkey = edit.transcription_hotkey.trim().to_string();
    settings.translation_hotkey = edit.translation_hotkey.trim().to_string();

    let active_profile_id = settings.active_api_profile_id.clone();
    if let Some(profile) = settings
        .api_profiles
        .iter_mut()
        .find(|profile| profile.id == active_profile_id)
    {
        profile.base_url = edit.base_url.trim().to_string();
        profile.model = edit.model.trim().to_string();
    }

    settings
}

fn api_key_for_profile<'a>(api_keys: &'a BTreeMap<String, String>, profile_id: &str) -> &'a str {
    api_keys
        .get(profile_id)
        .or_else(|| api_keys.get("default"))
        .map(String::as_str)
        .unwrap_or("")
}

fn model_for_request(model: &str) -> &str {
    non_empty_str(model).unwrap_or(AI2NPU_DEFAULT_MODEL)
}

fn non_empty_str(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn clear_logs(paths: &AppPaths) -> anyhow::Result<()> {
    let logs_dir = paths.logs_dir();
    if !logs_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&logs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn open_folder(path: &std::path::Path) -> anyhow::Result<()> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationState {
    Idle,
    Recording(AudioRequestKind),
    Transcribing(AudioRequestKind),
    Inserting,
    Error,
}

#[derive(Debug, Clone)]
pub struct VoiceInsertState {
    recording_mode: RecordingMode,
    silence_threshold: f32,
    silence_timeout_ms: u64,
    max_recording_ms: u64,
    state: OperationState,
    elapsed_recording_ms: u64,
    silent_for_ms: u64,
}

impl VoiceInsertState {
    pub fn new(
        recording_mode: RecordingMode,
        silence_threshold: f32,
        silence_timeout_ms: u64,
        max_recording_ms: u64,
    ) -> Self {
        Self {
            recording_mode,
            silence_threshold: silence_threshold.clamp(0.0, 1.0),
            silence_timeout_ms,
            max_recording_ms,
            state: OperationState::Idle,
            elapsed_recording_ms: 0,
            silent_for_ms: 0,
        }
    }

    pub fn state(&self) -> OperationState {
        self.state
    }

    pub fn hotkey_pressed(&mut self, request_kind: AudioRequestKind) -> StateCommand {
        match (self.recording_mode, self.state) {
            (RecordingMode::Hold, OperationState::Idle)
            | (RecordingMode::Toggle | RecordingMode::SilenceTimeout, OperationState::Idle) => {
                self.start_recording(request_kind);
                StateCommand::StartRecording(request_kind)
            }
            (
                RecordingMode::Toggle | RecordingMode::SilenceTimeout,
                OperationState::Recording(_),
            ) => self.stop_for_transcription(),
            _ => StateCommand::None,
        }
    }

    pub fn hotkey_released(&mut self) -> StateCommand {
        if self.recording_mode == RecordingMode::Hold
            && matches!(self.state, OperationState::Recording(_))
        {
            return self.stop_for_transcription();
        }

        StateCommand::None
    }

    pub fn level_changed(&mut self, level: f32, delta_ms: u64) -> StateCommand {
        if !matches!(self.state, OperationState::Recording(_)) {
            return StateCommand::None;
        }

        self.elapsed_recording_ms = self.elapsed_recording_ms.saturating_add(delta_ms);
        if self.elapsed_recording_ms >= self.max_recording_ms {
            return self.stop_for_transcription();
        }

        if !should_stop_on_silence(self.recording_mode) {
            return StateCommand::None;
        }

        if level <= self.silence_threshold {
            self.silent_for_ms = self.silent_for_ms.saturating_add(delta_ms);
            if self.silent_for_ms >= self.silence_timeout_ms {
                return self.stop_for_transcription();
            }
        } else {
            self.silent_for_ms = 0;
        }

        StateCommand::None
    }

    pub fn mark_inserting(&mut self) {
        self.state = OperationState::Inserting;
    }

    pub fn mark_idle(&mut self) {
        self.state = OperationState::Idle;
    }

    pub fn mark_error(&mut self) {
        self.state = OperationState::Error;
    }

    fn start_recording(&mut self, request_kind: AudioRequestKind) {
        self.state = OperationState::Recording(request_kind);
        self.elapsed_recording_ms = 0;
        self.silent_for_ms = 0;
    }

    fn stop_for_transcription(&mut self) -> StateCommand {
        if let OperationState::Recording(kind) = self.state {
            self.state = OperationState::Transcribing(kind);
            return StateCommand::StopAndTranscribe(kind);
        }

        StateCommand::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateCommand {
    None,
    StartRecording(AudioRequestKind),
    StopAndTranscribe(AudioRequestKind),
}

#[derive(Debug)]
pub struct SingleInstanceGuard {
    acquired: bool,
    #[cfg(windows)]
    handle: HANDLE,
}

impl SingleInstanceGuard {
    pub fn acquire(name: &str) -> anyhow::Result<Self> {
        acquire_single_instance(name)
    }

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
    use super::{
        OperationState, SingleInstanceGuard, StateCommand, VoiceInsertState, api_key_for_profile,
        apply_settings_edit, clear_logs, model_for_request,
    };
    use crate::api::transcription::AudioRequestKind;
    use crate::paths::AppPaths;
    use crate::settings::RecordingMode;
    use crate::ui::SettingsEdit;
    use std::collections::BTreeMap;

    #[test]
    fn single_instance_guard_reports_acquired_on_non_conflicting_name() {
        let name = format!("VoiceInsertTestMutex{}", std::process::id());
        let guard = SingleInstanceGuard::acquire(&name).unwrap();

        assert!(guard.acquired());
    }

    #[test]
    fn toggle_press_starts_then_stops_recording() {
        let mut state = VoiceInsertState::new(RecordingMode::Toggle, 0.04, 1200, 120_000);

        assert_eq!(
            state.hotkey_pressed(AudioRequestKind::Transcription),
            StateCommand::StartRecording(AudioRequestKind::Transcription)
        );
        assert_eq!(
            state.hotkey_pressed(AudioRequestKind::Transcription),
            StateCommand::StopAndTranscribe(AudioRequestKind::Transcription)
        );
    }

    #[test]
    fn hold_release_stops_recording() {
        let mut state = VoiceInsertState::new(RecordingMode::Hold, 0.04, 1200, 120_000);

        state.hotkey_pressed(AudioRequestKind::Translation);

        assert_eq!(
            state.hotkey_released(),
            StateCommand::StopAndTranscribe(AudioRequestKind::Translation)
        );
    }

    #[test]
    fn toggle_ignores_silence_timeout() {
        let mut state = VoiceInsertState::new(RecordingMode::Toggle, 0.5, 100, 120_000);

        state.hotkey_pressed(AudioRequestKind::Transcription);

        assert_eq!(state.level_changed(0.0, 1000), StateCommand::None);
        assert_eq!(
            state.state(),
            OperationState::Recording(AudioRequestKind::Transcription)
        );
    }

    #[test]
    fn silence_timeout_mode_stops_on_silence() {
        let mut state = VoiceInsertState::new(RecordingMode::SilenceTimeout, 0.5, 100, 120_000);

        state.hotkey_pressed(AudioRequestKind::Transcription);

        assert_eq!(
            state.level_changed(0.0, 100),
            StateCommand::StopAndTranscribe(AudioRequestKind::Transcription)
        );
    }

    #[test]
    fn max_duration_stops_all_recording_modes() {
        let mut state = VoiceInsertState::new(RecordingMode::Hold, 0.5, 1000, 250);

        state.hotkey_pressed(AudioRequestKind::Transcription);

        assert_eq!(
            state.level_changed(1.0, 250),
            StateCommand::StopAndTranscribe(AudioRequestKind::Transcription)
        );
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
    fn empty_model_uses_transcription_fallback() {
        assert_eq!(model_for_request(""), "openai/whisper-large-v3-turbo");
        assert_eq!(model_for_request(" custom-model "), "custom-model");
    }

    #[test]
    fn clear_logs_removes_files_in_logs_dir_only() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(temp.path());
        std::fs::create_dir_all(paths.logs_dir()).unwrap();
        let log_path = paths.logs_dir().join("voiceinsert.log");
        std::fs::write(&log_path, "log").unwrap();

        clear_logs(&paths).unwrap();

        assert!(!log_path.exists());
        assert!(paths.logs_dir().exists());
    }

    #[test]
    fn apply_settings_edit_updates_active_profile_and_hotkeys() {
        let settings = crate::settings::AppSettings::default().normalized();
        let settings = apply_settings_edit(
            settings,
            SettingsEdit {
                base_url: " http://localhost:9555 ".to_string(),
                model: " whisper-large-v3 ".to_string(),
                transcription_hotkey: " Ctrl+Space ".to_string(),
                translation_hotkey: " Alt+Y ".to_string(),
            },
        )
        .normalized();

        assert_eq!(settings.active_profile().base_url, "http://localhost:9555");
        assert_eq!(settings.active_profile().model, "whisper-large-v3");
        assert_eq!(settings.hotkey, "Ctrl+Space");
        assert_eq!(settings.translation_hotkey, "Alt+Y");
    }
}
