use voiceinsert::audio::levels::{peak_level_i16_le, should_stop_on_silence};
use voiceinsert::audio::recorder::{
    AudioDevice, append_limited_samples, convert_f32_to_i16, convert_f64_to_i16, convert_i8_to_i16,
    convert_i24_to_i16, convert_i32_to_i16, convert_i64_to_i16, convert_u8_to_i16,
    convert_u16_to_i16, convert_u32_to_i16, convert_u64_to_i16, encode_wav_mono_16khz_i16,
    resample_linear_i16,
};
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

#[test]
fn wav_encoding_matches_ai2npu_required_format() {
    let bytes = encode_wav_mono_16khz_i16(&[0, i16::MAX, i16::MIN]).unwrap();

    assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        16_000
    );
    assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
}

#[test]
fn resampling_converts_common_capture_rate_to_ai2npu_rate() {
    let source = vec![0; 48_000];
    let output = resample_linear_i16(&source, 48_000, 16_000);

    assert_eq!(output.len(), 16_000);
}

#[test]
fn sample_conversions_cover_full_scale_values() {
    assert_eq!(convert_i8_to_i16(i8::MIN), i16::MIN);
    assert_eq!(convert_i8_to_i16(i8::MAX), i16::MAX);
    assert_eq!(
        convert_i24_to_i16(cpal::I24::new((1 << 23) - 1).unwrap()),
        i16::MAX
    );
    assert_eq!(convert_i32_to_i16(i32::MIN), i16::MIN);
    assert_eq!(convert_i32_to_i16(i32::MAX), i16::MAX);
    assert_eq!(convert_i64_to_i16(i64::MIN), i16::MIN);
    assert_eq!(convert_i64_to_i16(i64::MAX), i16::MAX);
    assert_eq!(convert_u8_to_i16(0), i16::MIN);
    assert_eq!(convert_u8_to_i16(u8::MAX), i16::MAX);
    assert_eq!(convert_u16_to_i16(0), i16::MIN);
    assert_eq!(convert_u16_to_i16(u16::MAX), i16::MAX);
    assert_eq!(convert_u32_to_i16(0), i16::MIN);
    assert_eq!(convert_u32_to_i16(u32::MAX), i16::MAX);
    assert_eq!(convert_u64_to_i16(0), i16::MIN);
    assert_eq!(convert_u64_to_i16(u64::MAX), i16::MAX);
    assert_eq!(convert_f32_to_i16(-1.0), i16::MIN);
    assert_eq!(convert_f32_to_i16(1.0), i16::MAX);
    assert_eq!(convert_f64_to_i16(-1.0), i16::MIN);
    assert_eq!(convert_f64_to_i16(1.0), i16::MAX);
}

#[test]
fn append_limited_samples_keeps_most_recent_samples() {
    let mut buffer = vec![1, 2, 3];

    append_limited_samples(&mut buffer, &[4, 5, 6], 4);

    assert_eq!(buffer, vec![3, 4, 5, 6]);
}
