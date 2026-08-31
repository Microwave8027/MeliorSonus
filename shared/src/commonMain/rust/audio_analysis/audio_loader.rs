use std::error::Error;

/// Decoded audio buffer containing PCM samples and stream metadata.
#[derive(Clone, Debug)]
pub struct DecodedAudioTrack {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f32,
    pub samples_mono: Vec<f32>,
}

/// Offline audio decoder (stubbed for `symphonia` crate).
pub struct AudioLoader;

impl AudioLoader {
    /// Decodes an audio file (MP3, AAC, FLAC, WAV, Vorbis) from memory bytes into 32-bit float mono PCM.
    pub fn decode_bytes(_data: &[u8]) -> Result<DecodedAudioTrack, Box<dyn Error>> {
        // Handoff stub for symphonia:
        // let mss = symphonia::core::io::MediaSourceStream::new(Box::new(Cursor::new(data)), Default::default());
        // let probed = symphonia::default::get_probe().format(&Default::default(), mss, &Default::default(), &Default::default())?;
        // Decode packets into float PCM samples and average channels to mono.
        Ok(DecodedAudioTrack {
            sample_rate: 44100,
            channels: 1,
            duration_seconds: 0.0,
            samples_mono: Vec::new(),
        })
    }

    /// Decodes an audio file directly from a filesystem path.
    pub fn decode_file(_path: &str) -> Result<DecodedAudioTrack, Box<dyn Error>> {
        // Handoff stub:
        // let bytes = std::fs::read(path)?;
        // Self::decode_bytes(&bytes)
        Ok(DecodedAudioTrack {
            sample_rate: 44100,
            channels: 1,
            duration_seconds: 0.0,
            samples_mono: Vec::new(),
        })
    }
}
