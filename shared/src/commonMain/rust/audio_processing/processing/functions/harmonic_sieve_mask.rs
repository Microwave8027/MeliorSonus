use crate::constants::*;

/// Pre-allocated Harmonic Sieve Layer for piano harmonic disentanglement
pub struct HarmonicSieveMasker {
    inharmonicity_b: f32,
    max_harmonics: usize,
    bin_centers: [f32; PITCH_BINS],
    ghost_mask: [bool; PITCH_BINS],
}

impl HarmonicSieveMasker {
    pub fn new(inharmonicity_b: f32, max_harmonics: usize) -> Self {
        let mut bin_centers = [0.0f32; PITCH_BINS];
        for (i, center) in bin_centers.iter_mut().enumerate() {
            let midi_note = (i + 21) as f32; // MIDI 21 = A0 
            *center = 440.0 * 2.0f32.powf((midi_note - 69.0) / 12.0);
        }

        Self {
            inharmonicity_b,
            max_harmonics,
            bin_centers,
            ghost_mask: [false; PITCH_BINS],
        }
    }

    /// Disentangles raw CRNN pitch activations using bin-relative inharmonic comb masking
    pub fn apply_sieve(
        &mut self,
        raw_probs: &[f32; PITCH_BINS],
        _sample_rate: f32,
        out_sieved_probs: &mut [f32; PITCH_BINS],
    ) {
        self.ghost_mask.fill(false);
        out_sieved_probs.copy_from_slice(raw_probs);

        // Identify candidate fundamentals and check for octave harmonic ghosts (2f0, 3f0, ...)
        for i in 0..PITCH_BINS {
            if self.ghost_mask[i] || raw_probs[i] < 0.40 {
                continue;
            }

            let f0 = self.bin_centers[i];

            for k in 2..=self.max_harmonics {
                let fk = (k as f32) * f0 * (1.0 + self.inharmonicity_b * (k as f32).powi(2)).sqrt();
                let harmonic_midi = 69.0 + 12.0 * (fk / 440.0).log2();
                let harmonic_idx = (harmonic_midi.round() as i32) - 21;

                if harmonic_idx >= 0 && (harmonic_idx as usize) < PITCH_BINS {
                    let h_idx = harmonic_idx as usize;
                    // Harmonic overtone acoustic energy decreases with harmonic index k.
                    // Only mask as ghost if candidate activation is within expected acoustic leakage
                    // and below the high-confidence note threshold (>= 0.65 indicates genuine chord note).
                    let leak_ceiling = raw_probs[i] * (0.80f32).powi((k - 1) as i32);
                    if raw_probs[h_idx] > 0.15 && raw_probs[h_idx] <= leak_ceiling && raw_probs[h_idx] < 0.65 {
                        self.ghost_mask[h_idx] = true;
                    }
                }
            }
        }

        // Mask out detected octave ghost harmonics
        for (out_prob, &is_ghost) in out_sieved_probs.iter_mut().zip(self.ghost_mask.iter()) {
            if is_ghost {
                *out_prob = 0.0;
            }
        }
    }
}
