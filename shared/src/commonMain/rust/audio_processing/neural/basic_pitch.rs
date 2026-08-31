use crate::audio_processing::neural::crnn::{BasicPitchOutput, NeuralTranscriber};
use crate::audio_processing::neural::litert_model::{
    HardwareDelegate, LiteRtBasicPitchModel, PowerMode,
};
use crate::audio_processing::neural::tract_model::TractBasicPitchModel;
use std::error::Error;
use std::path::Path;

/// Hybrid Neural Pitch Detection Model enum supporting both the LiteRT C FFI bridge
/// (primary hardware-accelerated backend on mobile) and the pure-Rust Tract ONNX runtime (CPU fallback).
pub enum PitchDetectionModel {
    LiteRt(LiteRtBasicPitchModel),
    Tract(TractBasicPitchModel),
}

pub type BasicPitchModel = PitchDetectionModel;
pub type BasicPitchRuntime = PitchDetectionModel;

impl PitchDetectionModel {
    /// Attempts to load the LiteRT model via C bridge first; if that fails or is unsupported on the current platform,
    /// falls back to loading the ONNX model via Tract runtime.
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        tflite_model_path: P1,
        onnx_model_path: P2,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let threads = device.recommended_threads(PowerMode::default());
        match LiteRtBasicPitchModel::from_file(tflite_model_path.as_ref(), device, threads) {
            Ok(litert_model) => Ok(Self::LiteRt(litert_model)),
            Err(_) => {
                let tract_model = TractBasicPitchModel::new(onnx_model_path.as_ref())?;
                Ok(Self::Tract(tract_model))
            }
        }
    }

    /// Loads specifically using the LiteRT C bridge.
    pub fn from_litert_file<P: AsRef<Path>>(
        tflite_model_path: P,
        device: HardwareDelegate,
        num_threads: i32,
    ) -> Result<Self, Box<dyn Error>> {
        let model = LiteRtBasicPitchModel::from_file(tflite_model_path, device, num_threads)?;
        Ok(Self::LiteRt(model))
    }

    /// Loads specifically using the Tract ONNX runtime.
    pub fn from_tract_file<P: AsRef<Path>>(onnx_model_path: P) -> Result<Self, Box<dyn Error>> {
        let model = TractBasicPitchModel::new(onnx_model_path)?;
        Ok(Self::Tract(model))
    }

    /// Loads from byte buffers, attempting LiteRT first and falling back to Tract.
    pub fn from_bytes(
        tflite_bytes: Option<&[u8]>,
        onnx_bytes: Option<&[u8]>,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let threads = device.recommended_threads(PowerMode::default());
        if let Some(bytes) = tflite_bytes {
            if let Ok(litert_model) = LiteRtBasicPitchModel::from_bytes(bytes, device, threads) {
                return Ok(Self::LiteRt(litert_model));
            }
        }
        if let Some(bytes) = onnx_bytes {
            let tract_model = TractBasicPitchModel::from_bytes(bytes)?;
            return Ok(Self::Tract(tract_model));
        }
        Err("Failed to initialize PitchDetectionModel from bytes: no valid model data provided or initialization failed".into())
    }

    /// Creates a stub instance for testing/fallback.
    pub fn new_stub() -> Self {
        Self::Tract(TractBasicPitchModel::new_stub())
    }
}

impl Default for PitchDetectionModel {
    fn default() -> Self {
        Self::new_stub()
    }
}

impl NeuralTranscriber for PitchDetectionModel {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput {
        match self {
            Self::LiteRt(model) => model.transcribe_hop(audio_hop_22k, loudness),
            Self::Tract(model) => model.transcribe_hop(audio_hop_22k, loudness),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            Self::LiteRt(model) => model.sample_rate(),
            Self::Tract(model) => model.sample_rate(),
        }
    }

    fn reset(&mut self) {
        match self {
            Self::LiteRt(model) => model.reset(),
            Self::Tract(model) => model.reset(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_detection_model_stub() {
        let mut model = PitchDetectionModel::new_stub();
        assert_eq!(model.sample_rate(), 22050);

        let audio = [0.0f32; 512];
        let output = model.transcribe_hop(audio.as_slice(), -25.0);
        assert_eq!(output.energy_dbfs, -25.0);
    }

    #[test]
    fn test_pitch_detection_model_fallback() {
        // Passing invalid LiteRT and ONNX paths should fail gracefully with Error
        let result = PitchDetectionModel::new(
            "invalid_path.tflite",
            "invalid_path.onnx",
            HardwareDelegate::Cpu,
        );
        assert!(result.is_err());
    }
}
