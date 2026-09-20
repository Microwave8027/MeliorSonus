use crate::audio_processing::neural::crnn::{
    BasicPitchOutput, NUM_PITCH_BINS, NeuralTranscriber,
};
use crate::audio_processing::neural::litert_model::{
    HardwareDelegate, LiteRtEngine, PowerMode,
};
use crate::audio_processing::neural::tract_model::TractEngine;
use std::error::Error;
use std::path::Path;

/// Hybrid Neural Pitch Detection Model enum supporting both the LiteRT C FFI bridge
/// (primary hardware-accelerated backend on mobile) and the pure-Rust Tract ONNX runtime (CPU fallback).
pub enum PitchDetectionModel {
    LiteRt(LiteRtEngine),
    Tract(TractEngine),
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
        match LiteRtEngine::from_file(tflite_model_path.as_ref(), device, threads) {
            Ok(litert_engine) => Ok(Self::LiteRt(litert_engine)),
            Err(_) => {
                let tract_engine = TractEngine::from_file(onnx_model_path.as_ref())?;
                Ok(Self::Tract(tract_engine))
            }
        }
    }

    /// Loads specifically using the LiteRT C bridge.
    pub fn from_litert_file<P: AsRef<Path>>(
        tflite_model_path: P,
        device: HardwareDelegate,
        num_threads: i32,
    ) -> Result<Self, Box<dyn Error>> {
        let engine = LiteRtEngine::from_file(tflite_model_path, device, num_threads)?;
        Ok(Self::LiteRt(engine))
    }

    /// Loads specifically using the Tract ONNX runtime.
    pub fn from_tract_file<P: AsRef<Path>>(onnx_model_path: P) -> Result<Self, Box<dyn Error>> {
        let engine = TractEngine::from_file(onnx_model_path)?;
        Ok(Self::Tract(engine))
    }

    /// Loads from byte buffers, attempting LiteRT first and falling back to Tract.
    pub fn from_bytes(
        tflite_bytes: Option<&[u8]>,
        onnx_bytes: Option<&[u8]>,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let threads = device.recommended_threads(PowerMode::default());
        if let Some(bytes) = tflite_bytes {
            if let Ok(litert_engine) = LiteRtEngine::from_bytes(bytes, device, threads) {
                return Ok(Self::LiteRt(litert_engine));
            }
        }
        if let Some(bytes) = onnx_bytes {
            let tract_engine = TractEngine::from_bytes(bytes)?;
            return Ok(Self::Tract(tract_engine))
        }
        Err("Failed to initialize PitchDetectionModel from bytes: no valid model data provided or initialization failed".into())
    }

    /// Creates a stub instance for testing/fallback.
    pub fn new_stub() -> Self {
        Self::Tract(TractEngine::new_stub())
    }
}

impl Default for PitchDetectionModel {
    fn default() -> Self {
        Self::new_stub()
    }
}

impl NeuralTranscriber for PitchDetectionModel {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput {
        let mut out = BasicPitchOutput::default();
        out.energy_dbfs = loudness;

        match self {
            Self::LiteRt(engine) => {
                let expected_bytes = engine.input_tensor_byte_size(0);
                let expected_samples = if expected_bytes > 0 {
                    expected_bytes / std::mem::size_of::<f32>()
                } else {
                    audio_hop_22k.len()
                };

                let status = if expected_samples == audio_hop_22k.len() {
                    engine.set_input_data(0, audio_hop_22k)
                } else if expected_samples > 0 && expected_samples != audio_hop_22k.len() {
                    let mut padded = [0.0f32; 1024];
                    if expected_samples <= padded.len() {
                        let copy_len = audio_hop_22k.len().min(expected_samples);
                        padded[..copy_len].copy_from_slice(&audio_hop_22k[..copy_len]);
                        engine.set_input_data(0, &padded[..expected_samples])
                    } else {
                        engine.set_input_data(0, audio_hop_22k)
                    }
                } else {
                    engine.set_input_data(0, audio_hop_22k)
                };

                if status.is_ok() && engine.invoke().is_ok() {
                    let _ = engine.get_output_data(0, &mut out.onsets);
                    let _ = engine.get_output_data(1, &mut out.frames);
                    let _ = engine.get_output_data(2, &mut out.contours);
                }
            }
            Self::Tract(engine) => {
                if let Ok(results) = engine.run_1d(audio_hop_22k) {
                    // Head 0 ("onsets"): Note onsets probability matrix [1, 88] or [1, T, 88]
                    if let Some(onsets_tensor) = results.get(0) {
                        if let Ok(onsets_view) = onsets_tensor.as_slice::<f32>() {
                            if onsets_view.len() >= NUM_PITCH_BINS {
                                let start = onsets_view.len() - NUM_PITCH_BINS;
                                out.onsets
                                    .copy_from_slice(&onsets_view[start..start + NUM_PITCH_BINS]);
                            } else {
                                let len = onsets_view.len().min(NUM_PITCH_BINS);
                                out.onsets[..len].copy_from_slice(&onsets_view[..len]);
                            }
                        }
                    }

                    // Head 1 ("frames"): Polyphonic note pitch activations [1, 88] or [1, T, 88]
                    if let Some(frames_tensor) = results.get(1) {
                        if let Ok(frames_view) = frames_tensor.as_slice::<f32>() {
                            if frames_view.len() >= NUM_PITCH_BINS {
                                let start = frames_view.len() - NUM_PITCH_BINS;
                                out.frames
                                    .copy_from_slice(&frames_view[start..start + NUM_PITCH_BINS]);
                            } else {
                                let len = frames_view.len().min(NUM_PITCH_BINS);
                                out.frames[..len].copy_from_slice(&frames_view[..len]);
                            }
                        }
                    }

                    // Head 2 ("contours"): Microtonal pitch contours [1, 264] or [1, T, 264]
                    if let Some(contours_tensor) = results.get(2) {
                        if let Ok(contours_view) = contours_tensor.as_slice::<f32>() {
                            let num_contours = NUM_PITCH_BINS * 3;
                            if contours_view.len() >= num_contours {
                                let start = contours_view.len() - num_contours;
                                out.contours
                                    .copy_from_slice(&contours_view[start..start + num_contours]);
                            } else {
                                let len = contours_view.len().min(num_contours);
                                out.contours[..len].copy_from_slice(&contours_view[..len]);
                            }
                        }
                    }
                }
            }
        }

        out
    }

    fn sample_rate(&self) -> u32 {
        22050
    }

    fn reset(&mut self) {}
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
