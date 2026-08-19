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
        for i in 0..PITCH_BINS {
            let midi_note = (i + 21) as f32; // MIDI 21 = A0 
            bin_centers[i] = 440.0 * 2.0f32.powf((midi_note - 69.0) / 12.0);
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
            if raw_probs[i] < 0.40 {
                continue;
            }

            let f0 = self.bin_centers[i];

            for k in 2..=self.max_harmonics {
                let fk = (k as f32) * f0 * (1.0 + self.inharmonicity_b * (k as f32).powi(2)).sqrt();
                let harmonic_midi = 69.0 + 12.0 * (fk / 440.0).log2();
                let harmonic_idx = (harmonic_midi.round() as i32) - 21;

                if harmonic_idx >= 0 && (harmonic_idx as usize) < PITCH_BINS {
                    let h_idx = harmonic_idx as usize;
                    // If harmonic candidate has lower activation relative to expected fundamental leakage, mask ghost
                    if raw_probs[h_idx] > 0.15 && raw_probs[h_idx] <= raw_probs[i] * 0.85 {
                        self.ghost_mask[h_idx] = true;
                    }
                }
            }
        }

        // Mask out detected octave ghost harmonics
        for i in 0..PITCH_BINS {
            if self.ghost_mask[i] {
                out_sieved_probs[i] = 0.0;
            }
        }
    }
}
