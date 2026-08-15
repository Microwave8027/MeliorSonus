//deprecated please ignore
use crate::prelude::*;
use hound;

#[derive(Clone)]
pub struct DummyNoiseFilter(Arc<Mutex<Vec<f32>>>);

impl Dsp for DummyNoiseFilter {
    fn dsp(&self, buffer: [f32; 1024], cfg: StreamConfig) -> () {
        if let Ok(mut guard) = self.0.lock() {
            guard.extend_from_slice(buffer);
        }
    }
}

impl DummyNoiseFilter {
    pub fn new() -> Self {
        DummyNoiseFilter(Arc::new(Mutex::new(Vec::new())))
    }

    pub fn encode_to_wav_file(&self, config: StreamConfig) -> Result<Vec<u8>, Box<dyn Error>> {
        let samples = self.0.lock().map_err(|e| e.to_string())?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(
                &mut cursor,
                hound::WavSpec {
                    channels: config.channels as u16,
                    sample_rate: config.sample_rate,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )?;
            for &sample in samples.iter() {
                writer.write_sample(sample)?;
            }
            writer.finalize()?;
        }
        Ok(cursor.into_inner())
    }
}
