//deprecated ignore
use crate::{
    dsp::{Dsp, DummyNoiseFilter},
    prelude::*,
};

#[derive(uniffi::Object)]
pub struct RecWave {
    engine: Mutex<Option<AudioEngine>>,
    noise_filter: DummyNoiseFilter,
}

#[uniffi::export]
impl RecWave {
    #[uniffi::constructor]
    pub fn new() -> Result<Self, RustError> {
        let dummy = DummyNoiseFilter::new();
        let dummy_c = dummy.clone();
        let engine = AudioEngine::new(move |data| dummy_c.dsp(data))?;
        Ok(RecWave {
            engine: Mutex::new(Some(engine)),
            noise_filter: dummy,
        })
    }

    #[uniffi::method]
    pub fn stop_and_encode_to_wav(&self) -> Result<Vec<u8>, RustError> {
        let engine = self
            .engine
            .lock()
            .map_err(|e| e.to_string())?
            .take()
            .ok_or("engine is already stoppped")?;
        let config = engine.config();
        engine.end();
        let bytes = self.noise_filter.encode_to_wav_file(config)?;
        Ok(bytes)
    }
}
