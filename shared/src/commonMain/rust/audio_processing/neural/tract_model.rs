use crate::audio_processing::neural::crnn::{BasicPitchOutput, NUM_PITCH_BINS, NeuralTranscriber};
use std::error::Error;
use std::io::{Cursor, Read};
use std::path::Path;
use tract_onnx::prelude::*;

pub type TractPlan = TypedRunnableModel<TypedModel>;

/// Production ONNX runtime neural transcriber using tract-onnx.
/// Executes the Spotify Basic Pitch model head on resampled 22.05 kHz audio frames.
pub struct TractBasicPitchModel {
    sample_rate: u32,
    plan: Option<TractPlan>,
}

impl TractBasicPitchModel {
    /// Loads ONNX model weights from a file path.
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self, Box<dyn Error>> {
        let plan = tract_onnx::onnx()
            .model_for_path(model_path.as_ref())?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self {
            sample_rate: 22050,
            plan: Some(plan),
        })
    }

    /// Convenience alias for loading from a file path.
    pub fn from_file<P: AsRef<Path>>(model_path: P) -> Result<Self, Box<dyn Error>> {
        Self::new(model_path)
    }

    /// Loads ONNX model weights from an embedded or in-memory byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let mut cursor = Cursor::new(bytes);
        Self::from_reader(&mut cursor)
    }

    /// Loads ONNX model weights from any reader implementing `Read`.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        let plan = tract_onnx::onnx()
            .model_for_read(&mut reader)?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self {
            sample_rate: 22050,
            plan: Some(plan),
        })
    }

    /// Creates a stub / fallback instance with no loaded model plan.
    pub fn new_stub() -> Self {
        Self {
            sample_rate: 22050,
            plan: None,
        }
    }
}

impl Default for TractBasicPitchModel {
    fn default() -> Self {
        Self::new_stub()
    }
}

unsafe impl Send for TractBasicPitchModel {}
unsafe impl Sync for TractBasicPitchModel {}

impl NeuralTranscriber for TractBasicPitchModel {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput {
        let mut out = BasicPitchOutput::default();
        out.energy_dbfs = loudness;

        if let Some(plan) = &self.plan {
            // Convert audio slice into Tract Tensor formatted for Basic Pitch input [1, N]
            let arr = tract_ndarray::Array2::from_shape_fn((1, audio_hop_22k.len()), |(_, j)| {
                audio_hop_22k[j]
            });
            let tensor: Tensor = arr.into();

            if let Ok(results) = plan.run(tvec!(tensor.into())) {
                // Spotify Basic Pitch output heads:
                // Head 0 ("onsets"): Note onsets probability matrix [1, 88] or [1, T, 88]
                // Head 1 ("frames"): Polyphonic note pitch activations [1, 88] or [1, T, 88]
                // Head 2 ("contours"): Microtonal pitch contours [1, 264] or [1, T, 264]

                // 1. Extract onsets
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

                // 2. Extract frames
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

                // 3. Extract contours
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
        } else {
            out.onsets.fill(0.0);
            out.frames.fill(0.0);
            out.contours.fill(0.0);
        }

        out
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tract_stub_creation() {
        let mut model = TractBasicPitchModel::new_stub();
        assert_eq!(model.sample_rate(), 22050);

        let audio = [0.0f32; 512];
        let output = model.transcribe_hop(audio.as_slice(), -20.0);
        assert_eq!(output.energy_dbfs, -20.0);
        assert_eq!(output.onsets[0], 0.0);
        assert_eq!(output.frames[0], 0.0);
        assert_eq!(output.contours[0], 0.0);
    }

    #[test]
    fn test_tract_default() {
        let model = TractBasicPitchModel::default();
        assert_eq!(model.sample_rate(), 22050);
    }

    #[test]
    fn test_tract_invalid_path() {
        let result = TractBasicPitchModel::new("non_existent_model_file.onnx");
        assert!(result.is_err());
    }
}
