use voiceinsert::audio::levels::{peak_level_i16_le, should_stop_on_silence};
use voiceinsert::audio::recorder::{AudioDevice, encode_wav_mono_16khz_i16};
use voiceinsert::settings::RecordingMode;

#[test]
fn toggle_and_hold_do_not_stop_on_silence() {
    assert!(!should_stop_on_silence(RecordingMode::Toggle));
    assert!(!should_stop_on_silence(RecordingMode::Hold));
}

#[test]
fn silence_timeout_stops_on_silence() {
    assert!(should_stop_on_silence(RecordingMode::SilenceTimeout));
}

#[test]
fn peak_level_reads_signed_16bit_samples() {
    let bytes = [0x00, 0x00, 0xff, 0x7f];
    let level = peak_level_i16_le(&bytes);
    assert!(level > 0.99);
}

#[test]
fn peak_level_uses_i16_minimum_as_full_scale() {
    let bytes = [0x00, 0x80];

    assert_eq!(peak_level_i16_le(&bytes), 1.0);
}

#[test]
fn peak_level_ignores_trailing_partial_sample() {
    let bytes = [0x00, 0x40, 0xff];

    assert_eq!(peak_level_i16_le(&bytes), 0.5);
}

#[test]
fn audio_device_label_includes_index_and_name() {
    let device = AudioDevice::new(2, "Microphone Array".to_string());

    assert_eq!(device.label, "[2] Microphone Array");
}

#[test]
fn wav_encoding_writes_riff_wave_header() {
    let bytes = encode_wav_mono_16khz_i16(&[0, i16::MAX, i16::MIN]).unwrap();

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
}
