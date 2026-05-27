#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};

use crate::api::transcription::AudioRequestKind;
use crate::audio::levels::should_stop_on_silence;
use crate::hotkeys::service::{GlobalHotkeyEvents, HotkeyAction, HotkeyEvents};
use crate::logging::LastErrorState;
use crate::paths::AppPaths;
use crate::settings::RecordingMode;
use crate::sounds::{SoundKind, SoundService};
use crate::tray::{RuntimeTray, TrayCommand, TrayState};
use std::time::Duration;

pub fn run() -> anyhow::Result<()> {
    let _instance = SingleInstanceGuard::acquire("VoiceInsertAppMutex")?;
    if !_instance.acquired {
        return Ok(());
    }

    let runtime = AppRuntime::initialize()?;
    runtime.run()
}

pub struct AppRuntime {
    state: VoiceInsertState,
    tray: RuntimeTray,
    hotkeys: GlobalHotkeyEvents,
    sounds: SoundService,
    last_error: LastErrorState,
}

impl AppRuntime {
    pub fn initialize() -> anyhow::Result<Self> {
        let paths = AppPaths::new()?;
        let settings = crate::settings::load_settings(&paths)?;
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
        let hotkeys = GlobalHotkeyEvents::register(&settings.hotkey, &settings.translation_hotkey)?;
        let sounds = SoundService {
            enabled: settings.enable_sounds,
        };

        Ok(Self {
            state,
            tray,
            hotkeys,
            sounds,
            last_error: LastErrorState::default(),
        })
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        loop {
            pump_platform_events();

            if let Some(command) = self.tray.next_command()
                && self.handle_tray_command(command)?
            {
                return Ok(());
            }

            if let Some(action) = self.hotkeys.next_event() {
                self.handle_hotkey_action(action);
            }

            std::thread::sleep(Duration::from_millis(16));
        }
    }

    fn handle_tray_command(&mut self, command: TrayCommand) -> anyhow::Result<bool> {
        match command {
            TrayCommand::Settings => {
                tracing::info!("settings command received");
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
            HotkeyAction::Released => self.state.hotkey_released(),
        };

        if let Err(error) = self.apply_state_command(command) {
            let message = error.to_string();
            self.last_error.set(message.clone());
            tracing::error!(%message, "failed to handle hotkey action");
            self.state.mark_error();
            self.tray.set_state(TrayState::Error);
            let _ = self.sounds.play(SoundKind::Error);
        }
    }

    fn apply_state_command(&mut self, command: StateCommand) -> anyhow::Result<()> {
        match command {
            StateCommand::None => {}
            StateCommand::StartRecording(kind) => {
                tracing::info!(?kind, "recording started");
                self.tray.set_state(TrayState::Recording);
                self.sounds.play(SoundKind::Start)?;
            }
            StateCommand::StopAndTranscribe(kind) => {
                tracing::info!(?kind, "recording stopped");
                self.tray.set_state(TrayState::Transcribing);
                self.sounds.play(SoundKind::Stop)?;
            }
        }

        Ok(())
    }
}

#[cfg(windows)]
fn pump_platform_events() {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
    };

    unsafe {
        let mut message = MSG::default();
        while PeekMessageW(&mut message, HWND(std::ptr::null_mut()), 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

#[cfg(not(windows))]
fn pump_platform_events() {}

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
    use super::{OperationState, SingleInstanceGuard, StateCommand, VoiceInsertState};
    use crate::api::transcription::AudioRequestKind;
    use crate::settings::RecordingMode;

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
}
