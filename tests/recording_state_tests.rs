use voiceinsert::audio::levels::should_stop_on_silence;
use voiceinsert::settings::RecordingMode;

#[test]
fn silence_policy_matches_spec() {
    assert!(!should_stop_on_silence(RecordingMode::Toggle));
    assert!(!should_stop_on_silence(RecordingMode::Hold));
    assert!(should_stop_on_silence(RecordingMode::SilenceTimeout));
}
