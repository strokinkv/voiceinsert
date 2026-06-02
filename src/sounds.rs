use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{Context, bail};
use cpal::{
    FromSample, Sample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

/// Sound event emitted by the recording workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    Start,
    Stop,
    Error,
}

/// Returns the embedded WAV bytes for a sound event.
pub fn sound_bytes(kind: SoundKind) -> &'static [u8] {
    match kind {
        SoundKind::Start => include_bytes!("../assets/record-start.wav"),
        SoundKind::Stop => include_bytes!("../assets/record-stop.wav"),
        SoundKind::Error => include_bytes!("../assets/record-error.wav"),
    }
}

/// Plays VoiceInsert notification sounds when sound alerts are enabled.
#[derive(Clone)]
pub struct SoundService {
    pub enabled: bool,
    start: WavSamples,
    stop: WavSamples,
    error: WavSamples,
    output: Arc<Mutex<Option<SoundOutput>>>,
}

impl SoundService {
    /// Decodes embedded sound assets and creates a service with lazy output-device caching.
    pub fn new(enabled: bool) -> anyhow::Result<Self> {
        Ok(Self {
            enabled,
            start: decode_wav(sound_bytes(SoundKind::Start))?,
            stop: decode_wav(sound_bytes(SoundKind::Stop))?,
            error: decode_wav(sound_bytes(SoundKind::Error))?,
            output: Arc::new(Mutex::new(None)),
        })
    }

    /// Plays the requested sound, returning immediately after the output stream is started.
    pub fn play(&self, kind: SoundKind) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let output = self.output()?;
        let samples = match kind {
            SoundKind::Start => self.start.clone(),
            SoundKind::Stop => self.stop.clone(),
            SoundKind::Error => self.error.clone(),
        };
        let output_samples = prepare_samples_for_output(
            &samples,
            output.stream_config.sample_rate.0,
            usize::from(output.stream_config.channels),
        )?;
        let playback_duration = output_samples.duration();

        let stream = match output.supported_config.sample_format() {
            cpal::SampleFormat::I8 => {
                build_stream::<i8>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::I16 => {
                build_stream::<i16>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::I24 => {
                build_stream::<cpal::I24>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::I32 => {
                build_stream::<i32>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::I64 => {
                build_stream::<i64>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::U8 => {
                build_stream::<u8>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::U16 => {
                build_stream::<u16>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::U32 => {
                build_stream::<u32>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::U64 => {
                build_stream::<u64>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::F32 => {
                build_stream::<f32>(&output.device, &output.stream_config, output_samples)
            }
            cpal::SampleFormat::F64 => {
                build_stream::<f64>(&output.device, &output.stream_config, output_samples)
            }
            format => bail!("unsupported output sample format: {format:?}"),
        }?;

        stream.play().context("failed to start sound playback")?;
        thread::Builder::new()
            .name("voiceinsert-sound".to_string())
            .spawn(move || {
                let _stream = stream;
                thread::sleep(playback_duration + Duration::from_millis(50));
            })
            .context("failed to spawn sound playback thread")?;

        Ok(())
    }

    fn output(&self) -> anyhow::Result<SoundOutput> {
        let mut output = self.output.lock().expect("sound output lock poisoned");
        if output.is_none() {
            *output = Some(SoundOutput::default_output()?);
        }
        Ok(output.as_ref().expect("sound output initialized").clone())
    }
}

#[derive(Debug, Clone)]
struct WavSamples {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: usize,
}

#[derive(Clone)]
struct SoundOutput {
    device: cpal::Device,
    supported_config: cpal::SupportedStreamConfig,
    stream_config: cpal::StreamConfig,
}

impl SoundOutput {
    fn default_output() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default output audio device available")?;
        let supported_config = device
            .default_output_config()
            .context("failed to get default output audio config")?;
        let stream_config = supported_config.clone().into();

        Ok(Self {
            device,
            supported_config,
            stream_config,
        })
    }
}

impl WavSamples {
    fn duration(&self) -> Duration {
        if self.channels == 0 || self.sample_rate == 0 {
            return Duration::ZERO;
        }

        Duration::from_secs_f64(
            self.samples.len() as f64 / self.channels as f64 / self.sample_rate as f64,
        )
    }
}

fn decode_wav(bytes: &[u8]) -> anyhow::Result<WavSamples> {
    let mut reader =
        hound::WavReader::new(Cursor::new(bytes)).context("failed to read embedded WAV")?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels);

    if channels == 0 {
        bail!("embedded WAV has no channels");
    }

    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .context("failed to decode floating point WAV samples")?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|sample| sample.map(|sample| f32::from(sample) / f32::from(i16::MAX)))
            .collect::<Result<Vec<_>, _>>()
            .context("failed to decode 16-bit WAV samples")?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 32 => {
            let max_amplitude = ((1_i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|sample| sample as f32 / max_amplitude))
                .collect::<Result<Vec<_>, _>>()
                .context("failed to decode 32-bit WAV samples")?
        }
        _ => bail!(
            "unsupported embedded WAV sample format: {:?} {} bits",
            spec.sample_format,
            spec.bits_per_sample
        ),
    };

    Ok(WavSamples {
        samples,
        sample_rate: spec.sample_rate,
        channels,
    })
}

fn prepare_samples_for_output(
    source: &WavSamples,
    target_sample_rate: u32,
    target_channels: usize,
) -> anyhow::Result<WavSamples> {
    if source.sample_rate == 0 {
        bail!("embedded WAV has zero sample rate");
    }
    if source.channels == 0 {
        bail!("embedded WAV has no channels");
    }
    if target_sample_rate == 0 {
        bail!("output device has zero sample rate");
    }
    if target_channels == 0 {
        bail!("output device has no channels");
    }

    let source_frames = source.samples.len() / source.channels;
    let target_frames = source_frames * target_sample_rate as usize / source.sample_rate as usize;
    let mut samples = Vec::with_capacity(target_frames * target_channels);

    for target_frame in 0..target_frames {
        let source_frame = target_frame * source.sample_rate as usize / target_sample_rate as usize;
        let source_frame_start =
            source_frame.min(source_frames.saturating_sub(1)) * source.channels;

        for target_channel in 0..target_channels {
            let sample = if source.channels == target_channels {
                source.samples[source_frame_start + target_channel]
            } else if source.channels == 1 {
                source.samples[source_frame_start]
            } else if target_channels == 1 {
                let frame =
                    &source.samples[source_frame_start..source_frame_start + source.channels];
                frame.iter().sum::<f32>() / frame.len() as f32
            } else {
                let source_channel = target_channel.min(source.channels - 1);
                source.samples[source_frame_start + source_channel]
            };
            samples.push(sample.clamp(-1.0, 1.0));
        }
    }

    Ok(WavSamples {
        samples,
        sample_rate: target_sample_rate,
        channels: target_channels,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: WavSamples,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let mut position = 0;
    let data = samples.samples;
    let err_fn = |err| tracing::warn!(error = %err, "sound playback stream error");

    device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                write_output_data(output, &data, &mut position);
            },
            err_fn,
            None,
        )
        .context("failed to build sound playback stream")
}

fn write_output_data<T>(output: &mut [T], samples: &[f32], position: &mut usize)
where
    T: Sample + FromSample<f32>,
{
    for output_sample in output {
        let sample = samples.get(*position).copied().unwrap_or(0.0);
        *output_sample = T::from_sample(sample);
        *position += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundKind, SoundService, decode_wav, prepare_samples_for_output, sound_bytes};

    #[test]
    fn embedded_sounds_are_wav_files() {
        assert_eq!(&sound_bytes(SoundKind::Start)[0..4], b"RIFF");
        assert_eq!(&sound_bytes(SoundKind::Stop)[0..4], b"RIFF");
        assert_eq!(&sound_bytes(SoundKind::Error)[0..4], b"RIFF");
    }

    #[test]
    fn embedded_sounds_decode_without_audio_device() {
        for kind in [SoundKind::Start, SoundKind::Stop, SoundKind::Error] {
            let samples = decode_wav(sound_bytes(kind)).expect("embedded sound should decode");

            assert!(!samples.samples.is_empty());
            assert!(samples.sample_rate > 0);
            assert!(samples.channels > 0);
        }
    }

    #[test]
    fn service_new_caches_decoded_sounds_without_audio_device() {
        let service = SoundService::new(false).expect("embedded sounds should decode");

        assert!(!service.start.samples.is_empty());
        assert!(!service.stop.samples.is_empty());
        assert!(!service.error.samples.is_empty());
    }

    #[test]
    fn disabled_play_is_noop_without_audio_device() {
        let service = SoundService::new(false).expect("embedded sounds should decode");

        service
            .play(SoundKind::Start)
            .expect("disabled sound should not touch audio device");
    }

    #[test]
    fn prepare_samples_matches_output_format_without_audio_device() {
        let samples =
            decode_wav(sound_bytes(SoundKind::Start)).expect("embedded sound should decode");
        let output =
            prepare_samples_for_output(&samples, 48_000, 2).expect("samples should convert");

        assert_eq!(output.sample_rate, 48_000);
        assert_eq!(output.channels, 2);
        assert_eq!(output.samples.len() % 2, 0);
    }
}
