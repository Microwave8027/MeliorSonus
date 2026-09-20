use crate::audio_processing::neural::crnn::{
    BasicPitchOutput, NUM_PITCH_BINS, NeuralTranscriber,
};
use crate::audio_processing::neural::litert_model::{
    HardwareDelegate, LiteRtEngine, PowerMode,
};
use crate::audio_processing::neural::tract_model::TractEngine;
use std::error::Error;
use std::path::Path;

/// ByteDance CRNN 7-Tensor Output Data Structure
#[derive(Clone, Debug)]
pub struct ByteDanceCrnnOutput {
    /// Note onsets (transient activations) in range [0.0, 1.0] for 88 MIDI pitches
    pub onsets: [f32; NUM_PITCH_BINS],
    /// Note offsets (release activations) in range [0.0, 1.0] for 88 MIDI pitches
    pub offsets: [f32; NUM_PITCH_BINS],
    /// Note frame presence (polyphonic pitch activations) in range [0.0, 1.0]
    pub frames: [f32; NUM_PITCH_BINS],
    /// Note physical strike velocity regression in range [0.0, 128.0]
    pub velocity: [f32; NUM_PITCH_BINS],

    /// Sustain pedal onset transient probability in range [0.0, 1.0] (1x1 scalar)
    pub pedal_onset: f32,
    /// Sustain pedal release transient probability in range [0.0, 1.0] (1x1 scalar)
    pub pedal_offset: f32,
    /// Sustain pedal active frame presence probability in range [0.0, 1.0] (1x1 scalar)
    pub pedal_frame: f32,

    /// Frame RMS loudness in dBFS
    pub energy_dbfs: f32,
}

impl Default for ByteDanceCrnnOutput {
    fn default() -> Self {
        Self {
            onsets: [0.0; NUM_PITCH_BINS],
            offsets: [0.0; NUM_PITCH_BINS],
            frames: [0.0; NUM_PITCH_BINS],
            velocity: [0.0; NUM_PITCH_BINS],
            pedal_onset: 0.0,
            pedal_offset: 0.0,
            pedal_frame: 0.0,
            energy_dbfs: -120.0,
        }
    }
}

impl From<&ByteDanceCrnnOutput> for BasicPitchOutput {
    fn from(b: &ByteDanceCrnnOutput) -> Self {
        Self {
            onsets: b.onsets,
            frames: b.frames,
            contours: [0.0; NUM_PITCH_BINS * 3],
            energy_dbfs: b.energy_dbfs,
        }
    }
}

pub enum ByteDanceCrnnBackend {
    LiteRt(LiteRtEngine),
    Tract(TractEngine),
    Stub,
}

pub struct ByteDanceCrnnModel {
    sample_rate: u32,
    backend: ByteDanceCrnnBackend,
}

impl ByteDanceCrnnModel {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        tflite_model_path: P1,
        onnx_model_path: P2,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let threads = device.recommended_threads(PowerMode::default());
        match LiteRtEngine::from_file(tflite_model_path.as_ref(), device, threads) {
            Ok(engine) => Ok(Self {
                sample_rate: 16000,
                backend: ByteDanceCrnnBackend::LiteRt(engine),
            }),
            Err(_) => {
                let engine = TractEngine::from_file(onnx_model_path.as_ref())?;
                Ok(Self {
                    sample_rate: 16000,
                    backend: ByteDanceCrnnBackend::Tract(engine),
                })
            }
        }
    }

    pub fn from_litert_file<P: AsRef<Path>>(
        tflite_model_path: P,
        device: HardwareDelegate,
        num_threads: i32,
    ) -> Result<Self, Box<dyn Error>> {
        let engine = LiteRtEngine::from_file(tflite_model_path, device, num_threads)?;
        Ok(Self {
            sample_rate: 16000,
            backend: ByteDanceCrnnBackend::LiteRt(engine),
        })
    }

    pub fn from_tract_file<P: AsRef<Path>>(onnx_model_path: P) -> Result<Self, Box<dyn Error>> {
        let engine = TractEngine::from_file(onnx_model_path)?;
        Ok(Self {
            sample_rate: 16000,
            backend: ByteDanceCrnnBackend::Tract(engine),
        })
    }

    pub fn from_bytes(
        tflite_bytes: Option<&[u8]>,
        onnx_bytes: Option<&[u8]>,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let threads = device.recommended_threads(PowerMode::default());
        if let Some(bytes) = tflite_bytes {
            if let Ok(engine) = LiteRtEngine::from_bytes(bytes, device, threads) {
                return Ok(Self {
                    sample_rate: 16000,
                    backend: ByteDanceCrnnBackend::LiteRt(engine),
                });
            }
        }
        if let Some(bytes) = onnx_bytes {
            let engine = TractEngine::from_bytes(bytes)?;
            return Ok(Self {
                sample_rate: 16000,
                backend: ByteDanceCrnnBackend::Tract(engine),
            });
        }
        Err("Failed to initialize ByteDanceCrnnModel from bytes: no valid model data provided".into())
    }

    pub fn new_stub() -> Self {
        Self {
            sample_rate: 16000,
            backend: ByteDanceCrnnBackend::Stub,
        }
    }

    /// Transcribes a 16 kHz audio chunk and extracts all 7 output heads:
    /// Heads 0..3: note onsets, offsets, frames, velocity
    /// Heads 4..6: pedal onset, offset, frame
    pub fn transcribe_crnn_hop(&mut self, audio_hop_16k: &[f32], loudness: f32) -> ByteDanceCrnnOutput {
        let mut out = ByteDanceCrnnOutput::default();
        out.energy_dbfs = loudness;

        match &mut self.backend {
            ByteDanceCrnnBackend::LiteRt(engine) => {
                let expected_bytes = engine.input_tensor_byte_size(0);
                let expected_samples = if expected_bytes > 0 {
                    expected_bytes / std::mem::size_of::<f32>()
                } else {
                    audio_hop_16k.len()
                };

                let status = if expected_samples == audio_hop_16k.len() {
                    engine.set_input_data(0, audio_hop_16k)
                } else if expected_samples > 0 && expected_samples != audio_hop_16k.len() {
                    let mut padded = [0.0f32; 1024];
                    if expected_samples <= padded.len() {
                        let copy_len = audio_hop_16k.len().min(expected_samples);
                        padded[..copy_len].copy_from_slice(&audio_hop_16k[..copy_len]);
                        engine.set_input_data(0, &padded[..expected_samples])
                    } else {
                        engine.set_input_data(0, audio_hop_16k)
                    }
                } else {
                    engine.set_input_data(0, audio_hop_16k)
                };

                if status.is_ok() && engine.invoke().is_ok() {
                    // 4 Note Heads:
                    let _ = engine.get_output_data(0, &mut out.onsets);
                    let _ = engine.get_output_data(1, &mut out.offsets);
                    let _ = engine.get_output_data(2, &mut out.frames);
                    let _ = engine.get_output_data(3, &mut out.velocity);

                    // 3 Pedal Heads (1x1 scalars):
                    out.pedal_onset = engine.get_output_scalar_f32(4).unwrap_or(0.0);
                    out.pedal_offset = engine.get_output_scalar_f32(5).unwrap_or(0.0);
                    out.pedal_frame = engine.get_output_scalar_f32(6).unwrap_or(0.0);
                }
            }
            ByteDanceCrnnBackend::Tract(engine) => {
                if let Ok(results) = engine.run_1d(audio_hop_16k) {
                    // Head 0: Onsets [88]
                    if let Some(t) = results.get(0).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.onsets[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 1: Offsets [88]
                    if let Some(t) = results.get(1).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.offsets[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 2: Frames [88]
                    if let Some(t) = results.get(2).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.frames[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 3: Velocity [88]
                    if let Some(t) = results.get(3).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.velocity[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 4: Pedal Onset [1]
                    if let Some(t) = results.get(4).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_onset = t[t.len() - 1];
                        }
                    }
                    // Head 5: Pedal Offset [1]
                    if let Some(t) = results.get(5).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_offset = t[t.len() - 1];
                        }
                    }
                    // Head 6: Pedal Frame [1]
                    if let Some(t) = results.get(6).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_frame = t[t.len() - 1];
                        }
                    }
                }
            }
            ByteDanceCrnnBackend::Stub => {}
        }

        out
    }

    /// Transcribes a 4D Log-Mel spectrogram tensor [1, 1, T, 229] and extracts all 7 output heads:
    /// Heads 0..3: note onsets, offsets, frames, velocity
    /// Heads 4..6: pedal onset, offset, frame
    pub fn transcribe_spectrogram_frame(&mut self, log_mel_tensor: &[f32], loudness: f32) -> ByteDanceCrnnOutput {
        let mut out = ByteDanceCrnnOutput::default();
        out.energy_dbfs = loudness;

        match &mut self.backend {
            ByteDanceCrnnBackend::LiteRt(engine) => {
                if engine.set_input_data(0, log_mel_tensor).is_ok() && engine.invoke().is_ok() {
                    let _ = engine.get_output_data(0, &mut out.onsets);
                    let _ = engine.get_output_data(1, &mut out.offsets);
                    let _ = engine.get_output_data(2, &mut out.frames);
                    let _ = engine.get_output_data(3, &mut out.velocity);

                    out.pedal_onset = engine.get_output_scalar_f32(4).unwrap_or(0.0);
                    out.pedal_offset = engine.get_output_scalar_f32(5).unwrap_or(0.0);
                    out.pedal_frame = engine.get_output_scalar_f32(6).unwrap_or(0.0);
                }
            }
            ByteDanceCrnnBackend::Tract(engine) => {
                const MEL_BINS: usize = 229;
                let time_steps = if log_mel_tensor.len() >= MEL_BINS {
                    log_mel_tensor.len() / MEL_BINS
                } else {
                    1
                };
                if let Ok(results) = engine.run_4d(log_mel_tensor, (1, 1, time_steps, MEL_BINS)) {
                    // Head 0: Onsets [88]
                    if let Some(t) = results.get(0).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.onsets[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 1: Offsets [88]
                    if let Some(t) = results.get(1).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.offsets[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 2: Frames [88]
                    if let Some(t) = results.get(2).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.frames[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 3: Velocity [88]
                    if let Some(t) = results.get(3).and_then(|t| t.as_slice::<f32>().ok()) {
                        let len = t.len().min(NUM_PITCH_BINS);
                        let start = t.len().saturating_sub(NUM_PITCH_BINS);
                        out.velocity[..len].copy_from_slice(&t[start..start + len]);
                    }
                    // Head 4: Pedal Onset [1]
                    if let Some(t) = results.get(4).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_onset = t[t.len() - 1];
                        }
                    }
                    // Head 5: Pedal Offset [1]
                    if let Some(t) = results.get(5).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_offset = t[t.len() - 1];
                        }
                    }
                    // Head 6: Pedal Frame [1]
                    if let Some(t) = results.get(6).and_then(|t| t.as_slice::<f32>().ok()) {
                        if !t.is_empty() {
                            out.pedal_frame = t[t.len() - 1];
                        }
                    }
                }
            }
            ByteDanceCrnnBackend::Stub => {}
        }

        out
    }
}

impl Default for ByteDanceCrnnModel {
    fn default() -> Self {
        Self::new_stub()
    }
}

impl NeuralTranscriber for ByteDanceCrnnModel {
    fn transcribe_hop(&mut self, audio_hop_16k: &[f32], loudness: f32) -> BasicPitchOutput {
        let crnn_out = self.transcribe_crnn_hop(audio_hop_16k, loudness);
        BasicPitchOutput::from(&crnn_out)
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
    fn test_bytedance_crnn_stub_creation() {
        let mut model = ByteDanceCrnnModel::new_stub();
        assert_eq!(model.sample_rate(), 16000);

        let audio = [0.0f32; 512];
        let out = model.transcribe_crnn_hop(audio.as_slice(), -18.0);
        assert_eq!(out.energy_dbfs, -18.0);
        assert_eq!(out.onsets[0], 0.0);
        assert_eq!(out.offsets[0], 0.0);
        assert_eq!(out.frames[0], 0.0);
        assert_eq!(out.velocity[0], 0.0);
        assert_eq!(out.pedal_onset, 0.0);
        assert_eq!(out.pedal_offset, 0.0);
        assert_eq!(out.pedal_frame, 0.0);
    }

    #[test]
    fn test_neural_transcriber_trait_compatibility() {
        let mut model = ByteDanceCrnnModel::new_stub();
        let audio = [0.0f32; 512];
        let basic_pitch_out = model.transcribe_hop(audio.as_slice(), -15.0);
        assert_eq!(basic_pitch_out.energy_dbfs, -15.0);
        assert_eq!(basic_pitch_out.onsets[0], 0.0);
    }

    #[test]
    fn test_bytedance_crnn_spectrogram_frame_stub() {
        let mut model = ByteDanceCrnnModel::new_stub();
        let spectrogram = [0.0f32; 229 * 32];
        let out = model.transcribe_spectrogram_frame(&spectrogram, -20.0);
        assert_eq!(out.energy_dbfs, -20.0);
        assert_eq!(out.onsets[0], 0.0);
        assert_eq!(out.velocity[0], 0.0);
        assert_eq!(out.pedal_frame, 0.0);
    }

    #[test]
    fn test_bytedance_crnn_tract_file_load() {
        let path = std::path::Path::new("../tools/export_crnn/models/bytedance_crnn_acoustic_fp32.onnx");
        if path.exists() {
            let model_res = ByteDanceCrnnModel::from_tract_file(path);
            assert!(model_res.is_ok(), "Failed to load tract model {:?}: {:?}", path, model_res.err());
            let mut model = model_res.unwrap();
            let spectrogram = [0.0f32; 229 * 32];
            let out = model.transcribe_spectrogram_frame(&spectrogram, -20.0);
            assert_eq!(out.energy_dbfs, -20.0);
        }
    }
}
