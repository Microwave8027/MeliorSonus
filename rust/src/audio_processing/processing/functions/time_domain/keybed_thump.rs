use std::f32::consts::PI;

use crate::constants::{FRAME_SIZE, HOP_SIZE};

/// Zero-allocation keybed thump detector.
///
/// Filters sub-100Hz mechanical impact audio and measures its energy in dBFS.
/// Useful for identifying physical piano keybed slamming or acoustic instrument body knocks.
pub struct KeybedThumpDetector {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    s1: f32,
    s2: f32,
}

impl KeybedThumpDetector {
    pub fn new(sample_rate: u32) -> Self {
        let cutoff = 100.0f32;
        let omega = 2.0 * PI * cutoff / (sample_rate as f32);
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let b0 = (1.0 - cos_w) * 0.5;
        let b1 = 1.0 - cos_w;
        let b2 = (1.0 - cos_w) * 0.5;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w;
        let a2 = 1.0 - alpha;

        let inv_a0 = 1.0 / a0;

        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    /// Evaluates the sub-100Hz energy in dBFS for a frame of audio.
    #[inline(always)]
    pub fn evaluate_frame(&mut self, frame: &[f32; FRAME_SIZE]) -> f32 {
        if frame.is_empty() {
            return -180.0;
        }

        let mut sum_sq = 0.0f32;
        for &x in frame {
            let y = self.b0 * x + self.s1;
            self.s1 = self.b1 * x - self.a1 * y + self.s2;
            self.s2 = self.b2 * x - self.a2 * y;
            sum_sq += y * y;
        }

        let rms = (sum_sq / frame.len() as f32).sqrt();
        20.0 * (rms + 1e-9).log10()
    }

    /// Evaluates the sub-100Hz energy in dBFS for a hop of audio (HOP_SIZE).
    #[inline(always)]
    pub fn evaluate_hop(&mut self, hop: &[f32; HOP_SIZE]) -> f32 {
        if hop.is_empty() {
            return -180.0;
        }

        let mut sum_sq = 0.0f32;
        for &x in hop {
            let y = self.b0 * x + self.s1;
            self.s1 = self.b1 * x - self.a1 * y + self.s2;
            self.s2 = self.b2 * x - self.a2 * y;
            sum_sq += y * y;
        }

        let rms = (sum_sq / hop.len() as f32).sqrt();
        20.0 * (rms + 1e-9).log10()
    }
}
