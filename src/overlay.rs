use std::collections::VecDeque;

const MAX_WAVEFORM_POINTS: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayStatus {
    Recording,
    Transcribing,
    Inserting,
    Error,
}

#[derive(Debug, Clone)]
pub struct OverlayState {
    visible: bool,
    status: OverlayStatus,
    level: f32,
    is_silent: bool,
    waveform: VecDeque<f32>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            visible: false,
            status: OverlayStatus::Recording,
            level: 0.0,
            is_silent: false,
            waveform: VecDeque::with_capacity(MAX_WAVEFORM_POINTS),
        }
    }
}

impl OverlayState {
    pub fn show_recording(&mut self) {
        self.visible = true;
        self.status = OverlayStatus::Recording;
        self.reset_waveform();
    }

    pub fn set_level(&mut self, level: f32, silence_threshold: f32) {
        self.level = level.clamp(0.0, 1.0);
        self.is_silent = self.level <= silence_threshold.clamp(0.0, 1.0);
        if self.waveform.len() == MAX_WAVEFORM_POINTS {
            self.waveform.pop_front();
        }
        self.waveform.push_back(self.level);
    }

    pub fn set_status(&mut self, status: OverlayStatus) {
        self.status = status;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn reset_waveform(&mut self) {
        self.level = 0.0;
        self.is_silent = false;
        self.waveform.clear();
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn status(&self) -> OverlayStatus {
        self.status
    }

    pub fn waveform(&self) -> &VecDeque<f32> {
        &self.waveform
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_WAVEFORM_POINTS, OverlayState, OverlayStatus};

    #[test]
    fn show_recording_resets_waveform() {
        let mut overlay = OverlayState::default();
        overlay.set_level(0.8, 0.1);
        overlay.set_status(OverlayStatus::Error);

        overlay.show_recording();

        assert!(overlay.visible());
        assert_eq!(overlay.status(), OverlayStatus::Recording);
        assert!(overlay.waveform().is_empty());
    }

    #[test]
    fn waveform_keeps_latest_points() {
        let mut overlay = OverlayState::default();

        for _ in 0..(MAX_WAVEFORM_POINTS + 8) {
            overlay.set_level(0.5, 0.1);
        }

        assert_eq!(overlay.waveform().len(), MAX_WAVEFORM_POINTS);
    }
}
