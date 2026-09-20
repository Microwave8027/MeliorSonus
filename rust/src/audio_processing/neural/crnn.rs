pub const NUM_PITCH_BINS: usize = 88; // MIDI 21 (A0) to MIDI 108 (C8)
pub const MIDI_OFFSET: usize = 21;
pub const NUM_MIDI_NOTES: usize = 128; // Global MIDI notes 0 (C-1) to 127 (G9)

/// Inference output format corresponding to Spotify Basic Pitch model heads.
#[derive(Clone, Debug)]
pub struct BasicPitchOutput {
    /// Note onsets (transient activations) in range [0.0, 1.0] for 88 MIDI pitches
    pub onsets: [f32; NUM_PITCH_BINS],
    /// Note frame presence (polyphonic pitch activations) in range [0.0, 1.0]
    pub frames: [f32; NUM_PITCH_BINS],
    /// Fine pitch contours (3 bins per semitone, 264 bins total)
    pub contours: [f32; NUM_PITCH_BINS * 3],
    /// Frame RMS loudness in dBFS
    pub energy_dbfs: f32,
}

impl Default for BasicPitchOutput {
    fn default() -> Self {
        Self {
            onsets: [0.0; NUM_PITCH_BINS],
            frames: [0.0; NUM_PITCH_BINS],
            contours: [0.0; NUM_PITCH_BINS * 3],
            energy_dbfs: -120.0,
        }
    }
}

/// Abstract trait for neural pitch transcription backends (Tract ONNX, ONNX Runtime, or Stub).
pub trait NeuralTranscriber: Send {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput;
    fn sample_rate(&self) -> u32;
    fn reset(&mut self);
}

/// Lightweight deterministic mock transcriber for offline/unit tests without downloading ONNX weights.
pub struct BasicPitchStub {
    sample_rate: u32,
    prev_energy_dbfs: f32,
    active_midi_hint: Option<usize>,
}

impl BasicPitchStub {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            prev_energy_dbfs: -120.0,
            active_midi_hint: None,
        }
    }

    /// Helper to project detected frequency into the 88 MIDI output bins.
    fn freq_to_midi_index(&self, freq: f32) -> Option<usize> {
        if freq < 27.5 || freq > 4186.0 {
            return None;
        }
        let midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as usize;
        if midi >= MIDI_OFFSET && midi < MIDI_OFFSET + NUM_PITCH_BINS {
            Some(midi - MIDI_OFFSET)
        } else {
            None
        }
    }
}

impl NeuralTranscriber for BasicPitchStub {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput {
        let mut out = BasicPitchOutput::default();
        if audio_hop_22k.is_empty() {
            return out;
        }

        let energy = loudness;
        out.energy_dbfs = energy;

        if energy < -50.0 {
            self.prev_energy_dbfs = energy;
            self.active_midi_hint = None;
            return out;
        }

        // Estimate zero-crossings or approximate dominant frequency for the stub
        let mut zero_crossings = 0;
        for i in 1..audio_hop_22k.len() {
            if (audio_hop_22k[i - 1] >= 0.0 && audio_hop_22k[i] < 0.0)
                || (audio_hop_22k[i - 1] < 0.0 && audio_hop_22k[i] >= 0.0)
            {
                zero_crossings += 1;
            }
        }

        let approx_freq =
            (zero_crossings as f32 * self.sample_rate as f32) / (2.0 * audio_hop_22k.len() as f32);

        if let Some(midi_idx) = self.freq_to_midi_index(approx_freq) {
            let energy_norm: f32 = ((energy + 50.0) / 45.0).clamp(0.0, 1.0);

            let energy_delta = energy - self.prev_energy_dbfs;
            let mut is_onset = energy_delta > 3.0
                || (self.active_midi_hint.is_none() && energy > -45.0)
                || (self.active_midi_hint != Some(midi_idx)
                    && energy > -35.0
                    && energy_delta > -0.5);

            if energy_delta < -1.5 {
                is_onset = false;
            }

            if is_onset {
                let onset_val: f32 = 0.7f32 + 0.3f32 * energy_norm;
                out.onsets[midi_idx] = onset_val.clamp(0.0, 1.0);
            }

            let frame_val: f32 = 0.6f32 + 0.4f32 * energy_norm;
            out.frames[midi_idx] = frame_val.clamp(0.0, 1.0);

            self.active_midi_hint = Some(midi_idx);
        } else {
            self.active_midi_hint = None;
        }

        self.prev_energy_dbfs = energy;
        out
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn reset(&mut self) {
        self.prev_energy_dbfs = -120.0;
        self.active_midi_hint = None;
    }
}
