use crate::audio::levels::peak_level_i16;
use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{I24, SampleFormat, SampleRate, Stream, StreamConfig};
use hound::{SampleFormat as WavSampleFormat, WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub const TARGET_SAMPLE_RATE: u32 = 16_000;
pub const TARGET_CHANNELS: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub label: String,
}

impl AudioDevice {
    pub fn new(index: usize, name: String) -> Self {
        let label = format!("[{index}] {name}");
        Self { index, name, label }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecorderConfig {
    pub device_index: Option<usize>,
    pub sample_rate: u32,
    pub max_buffer_samples: usize,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            device_index: None,
            sample_rate: TARGET_SAMPLE_RATE,
            max_buffer_samples: TARGET_SAMPLE_RATE as usize * 3600,
        }
    }
}

pub struct Recorder {
    config: RecorderConfig,
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<i16>>>,
    capture_sample_rate: u32,
}

impl Recorder {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            config,
            stream: None,
            samples: Arc::new(Mutex::new(Vec::new())),
            capture_sample_rate: TARGET_SAMPLE_RATE,
        }
    }

    pub fn list_input_devices() -> anyhow::Result<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .context("failed to enumerate input devices")?;

        devices
            .enumerate()
            .map(|(index, device)| {
                let name = device
                    .name()
                    .unwrap_or_else(|_| format!("Input device {index}"));
                Ok(AudioDevice::new(index, name))
            })
            .collect()
    }

    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }

    pub fn start<F>(&mut self, mut on_level: F) -> anyhow::Result<()>
    where
        F: FnMut(f32) + Send + 'static,
    {
        if self.stream.is_some() {
            return Ok(());
        }

        self.samples
            .lock()
            .expect("recorder samples lock poisoned")
            .clear();

        let host = cpal::default_host();
        let device = match self.config.device_index {
            Some(index) => host
                .input_devices()
                .context("failed to enumerate input devices")?
                .nth(index)
                .with_context(|| format!("input device {index} was not found"))?,
            None => host
                .default_input_device()
                .context("no default input device is available")?,
        };

        let supported_config = supported_config(&device, self.config.sample_rate)?;
        let sample_format = supported_config.sample_format();
        let stream_config: StreamConfig = supported_config.into();
        let channels = usize::from(stream_config.channels);
        self.capture_sample_rate = stream_config.sample_rate.0;
        let samples = Arc::clone(&self.samples);
        let max_buffer_samples = self.config.max_buffer_samples;
        let err_fn = |error| tracing::warn!(%error, "input audio stream error");

        let stream = match sample_format {
            SampleFormat::I8 => device.build_input_stream(
                &stream_config,
                move |data: &[i8], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_i8_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_i16_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::I24 => device.build_input_stream(
                &stream_config,
                move |data: &[I24], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_i24_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::I32 => device.build_input_stream(
                &stream_config,
                move |data: &[i32], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_i32_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::I64 => device.build_input_stream(
                &stream_config,
                move |data: &[i64], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_i64_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::U8 => device.build_input_stream(
                &stream_config,
                move |data: &[u8], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_u8_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_u16_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::U32 => device.build_input_stream(
                &stream_config,
                move |data: &[u32], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_u32_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::U64 => device.build_input_stream(
                &stream_config,
                move |data: &[u64], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_u64_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_f32_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            SampleFormat::F64 => device.build_input_stream(
                &stream_config,
                move |data: &[f64], _| {
                    capture_samples(
                        data,
                        channels,
                        &samples,
                        &mut on_level,
                        convert_f64_to_i16,
                        max_buffer_samples,
                    );
                },
                err_fn,
                None,
            )?,
            format => anyhow::bail!("unsupported input sample format: {format:?}"),
        };

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<Vec<u8>> {
        self.stream.take();
        let samples = self
            .samples
            .lock()
            .expect("recorder samples lock poisoned")
            .clone();

        encode_wav_mono_i16(&samples, self.capture_sample_rate)
    }
}

pub fn encode_wav_mono_16khz_i16(samples: &[i16]) -> anyhow::Result<Vec<u8>> {
    encode_wav_mono_i16(samples, TARGET_SAMPLE_RATE)
}

pub fn encode_wav_mono_i16(samples: &[i16], sample_rate: u32) -> anyhow::Result<Vec<u8>> {
    let spec = WavSpec {
        channels: TARGET_CHANNELS,
        sample_rate,
        bits_per_sample: 16,
        sample_format: WavSampleFormat::Int,
    };
    let mut bytes = Vec::new();

    {
        let cursor = Cursor::new(&mut bytes);
        let mut writer = WavWriter::new(cursor, spec)?;

        for sample in samples {
            writer.write_sample(*sample)?;
        }

        writer.finalize()?;
    }

    Ok(bytes)
}

pub fn convert_i16_to_i16(sample: i16) -> i16 {
    sample
}

pub fn convert_i8_to_i16(sample: i8) -> i16 {
    signed_to_i16(i64::from(sample), 8)
}

pub fn convert_i24_to_i16(sample: I24) -> i16 {
    signed_to_i16(i64::from(sample.inner()), 24)
}

pub fn convert_i32_to_i16(sample: i32) -> i16 {
    signed_to_i16(i64::from(sample), 32)
}

pub fn convert_i64_to_i16(sample: i64) -> i16 {
    signed_to_i16(sample, 64)
}

pub fn convert_u8_to_i16(sample: u8) -> i16 {
    unsigned_to_i16(u64::from(sample), 8)
}

pub fn convert_u16_to_i16(sample: u16) -> i16 {
    unsigned_to_i16(u64::from(sample), 16)
}

pub fn convert_u32_to_i16(sample: u32) -> i16 {
    unsigned_to_i16(u64::from(sample), 32)
}

pub fn convert_u64_to_i16(sample: u64) -> i16 {
    unsigned_to_i16(sample, 64)
}

pub fn convert_f32_to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32768.0)
        .round()
        .clamp(i16::MIN as f32, i16::MAX as f32) as i16
}

pub fn convert_f64_to_i16(sample: f64) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32768.0)
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn signed_to_i16(sample: i64, bits: u32) -> i16 {
    let negative_full_scale = -(1_i128 << (bits - 1));
    let positive_full_scale = (1_i128 << (bits - 1)) - 1;
    let sample = i128::from(sample);

    if sample <= negative_full_scale {
        return i16::MIN;
    }
    if sample >= positive_full_scale {
        return i16::MAX;
    }

    if sample < 0 {
        ((sample as f64 / -(negative_full_scale as f64)) * 32768.0)
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
    } else {
        ((sample as f64 / positive_full_scale as f64) * f64::from(i16::MAX))
            .round()
            .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
    }
}

fn unsigned_to_i16(sample: u64, bits: u32) -> i16 {
    let midpoint = 1_u128 << (bits - 1);
    let signed = i128::from(sample) - midpoint as i128;
    signed_to_i16(signed as i64, bits)
}

fn supported_config(
    device: &cpal::Device,
    preferred_sample_rate: u32,
) -> anyhow::Result<cpal::SupportedStreamConfig> {
    let preferred_rate = SampleRate(preferred_sample_rate);
    let mut fallback = None;

    for config in device
        .supported_input_configs()
        .context("failed to query supported input configs")?
    {
        if fallback.is_none() {
            fallback = Some(config.with_max_sample_rate());
        }

        if config.min_sample_rate() <= preferred_rate && preferred_rate <= config.max_sample_rate()
        {
            return Ok(config.with_sample_rate(preferred_rate));
        }
    }

    fallback.context("input device has no supported stream configs")
}

fn capture_samples<T, F, C>(
    data: &[T],
    channels: usize,
    samples: &Arc<Mutex<Vec<i16>>>,
    on_level: &mut F,
    convert: C,
    max_buffer_samples: usize,
) where
    T: Copy,
    F: FnMut(f32),
    C: Fn(T) -> i16,
{
    let mono = downmix_to_mono(data, channels, convert);
    let level = peak_level_i16(&mono);
    append_limited_samples(
        &mut samples.lock().expect("recorder samples lock poisoned"),
        &mono,
        max_buffer_samples,
    );
    on_level(level);
}

pub fn append_limited_samples(buffer: &mut Vec<i16>, samples: &[i16], max_samples: usize) {
    if max_samples == 0 {
        buffer.clear();
        return;
    }

    buffer.extend_from_slice(samples);
    if buffer.len() > max_samples {
        let extra = buffer.len() - max_samples;
        buffer.drain(0..extra);
    }
}

fn downmix_to_mono<T, C>(data: &[T], channels: usize, convert: C) -> Vec<i16>
where
    T: Copy,
    C: Fn(T) -> i16,
{
    let channels = channels.max(1);
    data.chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|sample| i32::from(convert(*sample))).sum();
            (sum / i32::try_from(frame.len()).unwrap_or(1))
                .clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
        })
        .collect()
}
