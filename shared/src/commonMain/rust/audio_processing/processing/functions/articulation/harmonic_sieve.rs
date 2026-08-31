use crate::constants::PITCH_BINS;

/// Standalone zero-allocation Harmonic Sieve Masker.
///
/// Disentangles genuine simultaneous chord notes from ghost overtone activations:
/// 1. Finds active fundamental peaks above `peak_threshold`.
/// 2. Projects integer harmonics (2f0, 3f0, 4f0...) with exponentially decaying leakage factors.
/// 3. Zeroes out overtone bins whose activation is fully explained by spectral leakage,
///    while preserving true polyphonic chord notes exceeding the leakage mask.
pub struct HarmonicSieveMasker {
    decay_rate: f32,
    max_harmonics: usize,
}

impl HarmonicSieveMasker {
    pub fn new(decay_rate: f32, max_harmonics: usize) -> Self {
        Self {
            decay_rate,
            max_harmonics,
        }
    }

    /// Applies the harmonic sieve mask to raw pitch bin activations in-place or into `output_probs`.
    pub fn apply_sieve(
        &mut self,
        raw_probs: &[f32; PITCH_BINS],
        _sample_rate: f32,
        output_probs: &mut [f32; PITCH_BINS],
    ) {
        output_probs.copy_from_slice(raw_probs);

        let mut leakage_mask = [0.0f32; PITCH_BINS];

        // 1. Calculate projected harmonic leakage from every active fundamental
        for (bin, &p) in raw_probs.iter().enumerate() {
            if p > 0.40 {
                // Approximate MIDI note: MIDI 21 (A0) + bin
                let midi_f0 = 21.0 + (bin as f32);
                let f0_hz = 440.0 * 2.0f32.powf((midi_f0 - 69.0) / 12.0);

                // Project overtones (h = 2, 3, 4 ... max_harmonics)
                for h in 2..=self.max_harmonics {
                    let fh_hz = f0_hz * (h as f32);
                    let midi_h = 69.0 + 12.0 * (fh_hz / 440.0).log2();
                    let target_bin = (midi_h.round() as i32) - 21;

                    if (0..(PITCH_BINS as i32)).contains(&target_bin) {
                        let t_idx = target_bin as usize;
                        let weight = 1.0 / ((h as f32).powf(1.2));
                        let expected_leak = p * weight * (1.0 - self.decay_rate * (h as f32));
                        if expected_leak > leakage_mask[t_idx] {
                            leakage_mask[t_idx] = expected_leak;
                        }
                    }
                }
            }
        }

        // 2. Sieve out ghost overtone peaks whose energy is dominated by the leakage mask
        for (bin, &mask) in leakage_mask.iter().enumerate() {
            if mask > 0.0 {
                if raw_probs[bin] <= mask + 0.15 {
                    output_probs[bin] = 0.0;
                } else {
                    // True chord note: keep raw probability intact
                    output_probs[bin] = raw_probs[bin];
                }
            }
        }
    }
}

impl Default for HarmonicSieveMasker {
    fn default() -> Self {
        Self::new(0.0002, 6)
    }
}
