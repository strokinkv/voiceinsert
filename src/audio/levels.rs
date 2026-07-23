use crate::settings::RecordingMode;

pub fn should_stop_on_silence(mode: RecordingMode) -> bool {
    matches!(mode, RecordingMode::Hybrid | RecordingMode::SilenceTimeout)
}

pub fn peak_level_i16_le(bytes: &[u8]) -> f32 {
    let mut max = 0i32;

    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as i32;
        max = max.max(sample.abs());
    }

    (max as f32 / 32768.0).clamp(0.0, 1.0)
}

pub fn peak_level_i16(samples: &[i16]) -> f32 {
    let max = samples
        .iter()
        .map(|sample| (*sample as i32).abs())
        .max()
        .unwrap_or_default();

    (max as f32 / 32768.0).clamp(0.0, 1.0)
}

pub fn mean_absolute_level_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum: u64 = samples
        .iter()
        .map(|sample| u64::from((*sample as i32).unsigned_abs()))
        .sum();
    (sum as f32 / samples.len() as f32 / 32768.0).clamp(0.0, 1.0)
}
