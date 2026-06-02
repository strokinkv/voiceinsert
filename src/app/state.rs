use crate::api::transcription::AudioRequestKind;
use crate::audio::levels::should_stop_on_silence;
use crate::settings::RecordingMode;

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
    pub(super) silence_threshold: f32,
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

    #[cfg(test)]
    pub fn state(&self) -> OperationState {
        self.state
    }

    pub fn hotkey_pressed(&mut self, request_kind: AudioRequestKind) -> StateCommand {
        match (self.recording_mode, self.state) {
            (RecordingMode::Hold, OperationState::Idle)
            | (RecordingMode::Hold, OperationState::Error)
            | (RecordingMode::Toggle | RecordingMode::SilenceTimeout, OperationState::Idle)
            | (RecordingMode::Toggle | RecordingMode::SilenceTimeout, OperationState::Error) => {
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

    pub fn cancel_recording(&mut self) -> StateCommand {
        if matches!(self.state, OperationState::Recording(_)) {
            self.state = OperationState::Idle;
            self.elapsed_recording_ms = 0;
            self.silent_for_ms = 0;
            return StateCommand::CancelRecording;
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
    CancelRecording,
}

pub(super) fn log_error_message(error: &anyhow::Error) -> String {
    sanitize_log_text(&error.root_cause().to_string())
}

pub(super) fn sanitize_log_text(message: &str) -> String {
    let trimmed = message.lines().next().unwrap_or("").trim();
    trimmed.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::{OperationState, StateCommand, VoiceInsertState};
    use crate::api::transcription::AudioRequestKind;
    use crate::settings::RecordingMode;

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
    fn hotkey_can_start_new_recording_after_error() {
        let mut state = VoiceInsertState::new(RecordingMode::Toggle, 0.04, 1200, 120_000);

        state.mark_error();

        assert_eq!(
            state.hotkey_pressed(AudioRequestKind::Transcription),
            StateCommand::StartRecording(AudioRequestKind::Transcription)
        );
        assert_eq!(
            state.state(),
            OperationState::Recording(AudioRequestKind::Transcription)
        );
    }

    #[test]
    fn cancel_recording_returns_to_idle_without_transcribing() {
        let mut state = VoiceInsertState::new(RecordingMode::Toggle, 0.04, 1200, 120_000);

        assert_eq!(
            state.hotkey_pressed(AudioRequestKind::Transcription),
            StateCommand::StartRecording(AudioRequestKind::Transcription)
        );
        assert_eq!(state.cancel_recording(), StateCommand::CancelRecording);
        assert_eq!(state.state(), OperationState::Idle);
    }

    #[test]
    fn log_error_message_uses_root_cause_only() {
        let error = anyhow::anyhow!("base cause").context("outer context");

        assert_eq!(super::log_error_message(&error), "base cause");
    }

    #[test]
    fn sanitize_log_text_trims_newlines_and_limits_length() {
        let message = "first line\nsecond line";
        let sanitized = super::sanitize_log_text(message);

        assert_eq!(sanitized, "first line");
    }
}
