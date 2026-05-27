#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    Start,
    Stop,
    Error,
}

pub fn sound_bytes(kind: SoundKind) -> &'static [u8] {
    match kind {
        SoundKind::Start => include_bytes!("../assets/record-start.wav"),
        SoundKind::Stop => include_bytes!("../assets/record-stop.wav"),
        SoundKind::Error => include_bytes!("../assets/record-stop.wav"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundService {
    pub enabled: bool,
}

impl SoundService {
    pub fn play(&self, kind: SoundKind) -> anyhow::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let _ = sound_bytes(kind);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{SoundKind, sound_bytes};

    #[test]
    fn embedded_sounds_are_wav_files() {
        assert_eq!(&sound_bytes(SoundKind::Start)[0..4], b"RIFF");
        assert_eq!(&sound_bytes(SoundKind::Stop)[0..4], b"RIFF");
    }
}
