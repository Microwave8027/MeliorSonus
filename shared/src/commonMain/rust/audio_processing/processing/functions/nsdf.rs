use crate::prelude::*;

/// Standalone zero-allocation Normalized Square Difference Function (NSDF) evaluator.
///
/// Computes the Normalized Square Difference Function n'_t(tau) over a real-time audio frame:
///             2 * sum(x[n] * x[n + tau])
/// n'_t(tau) = -----------------------------------------
///             sum(x[n]^2) + sum(x[n + tau]^2)
///
/// Yields:
/// - `clarity` (r1): Highest periodicity peak value in NSDF (0.0 to 1.0)
/// - `secondary_peak_ratio` (r2 / r1): Ratio of secondary non-harmonic peak to primary peak
pub struct NsdfEvaluator {
    nsdf_buffer: [f32; FRAME_SIZE],
}

impl NsdfEvaluator {
    pub fn new() -> Self {
        Self {
            nsdf_buffer: [0.0; FRAME_SIZE],
        }
    }

    /// Evaluates NSDF monophonic clarity and secondary peak ratio in-place on audio thread.
    /// Returns `(clarity, secondary_peak_ratio)`.
    #[inline(always)]
    pub fn evaluate_frame(&mut self, frame: &[f32; FRAME_SIZE]) -> (f32, f32) {
        let n = FRAME_SIZE;
        let max_tau = n / 2;

        // 1. Compute NSDF n'_t(tau) for tau in 0..max_tau
        for tau in 0..max_tau {
            let mut num = 0.0f32;
            let mut den = 0.0f32;
            for j in 0..(n - tau) {
                let x1 = frame[j];
                let x2 = frame[j + tau];
                num += 2.0 * x1 * x2;
                den += x1 * x1 + x2 * x2;
            }
            self.nsdf_buffer[tau] = if den > 1e-9 { num / den } else { 0.0 };
        }

        // 2. Extract primary peak (r1) and secondary non-harmonic peak (r2)
        let (r1, r2) = self.find_primary_and_secondary_peaks(max_tau);
        let clarity = r1.max(0.0).min(1.0);
        let ratio = if r1 > 1e-4 { (r2 / r1).max(0.0) } else { 1.0 };

        (clarity, ratio)
    }

    fn find_primary_and_secondary_peaks(&self, max_tau: usize) -> (f32, f32) {
        let mut global_max = 0.0f32;
        const MIN_TAU: usize = 8; // Minimum lag threshold to prevent tau1 <= 3 suppression edge cases

        if max_tau <= MIN_TAU + 1 {
            return (0.0, 0.0);
        }

        // 1. Find global maximum peak magnitude M in NSDF starting at MIN_TAU
        for tau in MIN_TAU..(max_tau.saturating_sub(1)) {
            let prev = self.nsdf_buffer[tau - 1];
            let curr = self.nsdf_buffer[tau];
            let next = self.nsdf_buffer[tau + 1];

            if curr > prev && curr >= next && curr > global_max {
                global_max = curr;
            }
        }

        if global_max < 1e-4 {
            return (0.0, 0.0);
        }

        // 2. Fundamental period tau1 is the FIRST peak exceeding cutoff T_cut (0.80 * global_max) starting at MIN_TAU
        let cutoff = 0.80 * global_max;
        let mut r1 = 0.0f32;
        let mut peak_tau1: usize = 0;

        for tau in MIN_TAU..(max_tau.saturating_sub(1)) {
            let prev = self.nsdf_buffer[tau - 1];
            let curr = self.nsdf_buffer[tau];
            let next = self.nsdf_buffer[tau + 1];

            if curr > prev && curr >= next && curr >= cutoff {
                r1 = curr;
                peak_tau1 = tau;
                break;
            }
        }

        if peak_tau1 < MIN_TAU {
            return (0.0, 0.0);
        }

        // 3. Find highest secondary NON-HARMONIC peak r2 starting at MIN_TAU
        let mut r2 = 0.0f32;
        for tau in MIN_TAU..(max_tau.saturating_sub(1)) {
            let prev = self.nsdf_buffer[tau - 1];
            let curr = self.nsdf_buffer[tau];
            let next = self.nsdf_buffer[tau + 1];

            if curr > prev && curr >= next && curr > 0.0 {
                let k = ((tau as f32) / (peak_tau1 as f32)).round() as usize;
                let is_harmonic_multiple =
                    k >= 1 && (tau as i32 - (k * peak_tau1) as i32).abs() <= 3;

                if !is_harmonic_multiple && curr > r2 {
                    r2 = curr;
                }
            }
        }

        (r1, r2)
    }
}
