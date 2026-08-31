use crate::constants::*;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex, num_complex::Complex32};
use std::sync::Arc;

pub const FFT_SIZE: usize = FRAME_SIZE * 2;

/// Standalone zero-allocation Normalized Square Difference Function (NSDF) evaluator.
///
/// Computes the Normalized Square Difference Function n'_t(tau) over a real-time audio frame:
///             2 * sum(x[n] * x[n + tau])
/// n'_t(tau) = -----------------------------------------
///             sum(x[n]^2) + sum(x[n + tau]^2)
///
/// Uses O(N log N) SIMD forward/inverse RealFFT (Wiener-Khinchin theorem) and O(1) prefix-sum
/// cumulative energy tracking for zero-allocation, real-time safe performance.
///
/// Yields:
/// - `clarity` (r1): Highest periodicity peak value in NSDF (0.0 to 1.0)
/// - `secondary_peak_ratio` (r2 / r1): Ratio of secondary non-harmonic peak to primary peak
pub struct NsdfEvaluator {
    fft_forward: Arc<dyn RealToComplex<f32>>,
    fft_inverse: Arc<dyn ComplexToReal<f32>>,
    forward_input: Vec<f32>,
    forward_output: Vec<Complex32>,
    forward_scratch: Vec<Complex32>,
    inverse_input: Vec<Complex32>,
    inverse_output: Vec<f32>,
    inverse_scratch: Vec<Complex32>,
    sq_prefix: [f32; FRAME_SIZE + 1],
    nsdf_buffer: [f32; FRAME_SIZE],
}

impl NsdfEvaluator {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(FFT_SIZE);
        let fft_inverse = planner.plan_fft_inverse(FFT_SIZE);

        let forward_input = fft_forward.make_input_vec();
        let forward_output = fft_forward.make_output_vec();
        let forward_scratch = fft_forward.make_scratch_vec();

        let inverse_input = fft_inverse.make_input_vec();
        let inverse_output = fft_inverse.make_output_vec();
        let inverse_scratch = fft_inverse.make_scratch_vec();

        Self {
            fft_forward,
            fft_inverse,
            forward_input,
            forward_output,
            forward_scratch,
            inverse_input,
            inverse_output,
            inverse_scratch,
            sq_prefix: [0.0; FRAME_SIZE + 1],
            nsdf_buffer: [0.0; FRAME_SIZE],
        }
    }

    /// Evaluates NSDF monophonic clarity and secondary peak ratio in-place on audio thread in O(N log N) time.
    /// Returns `(clarity, secondary_peak_ratio)`.
    #[inline(always)]
    pub fn evaluate_frame(&mut self, frame: &[f32; FRAME_SIZE]) -> (f32, f32) {
        let n = FRAME_SIZE;
        let max_tau = n / 2;

        // 1. Calculate prefix sums of squared samples for O(1) denominator lookup
        self.sq_prefix[0] = 0.0;
        for i in 0..n {
            self.sq_prefix[i + 1] = self.sq_prefix[i] + frame[i] * frame[i];
        }

        let total_energy = self.sq_prefix[n];
        if total_energy < 1e-9 {
            self.nsdf_buffer.fill(0.0);
            return (0.0, 0.0);
        }

        // 2. Zero-pad frame to FFT_SIZE (2 * FRAME_SIZE) to prevent circular autocorrelation aliasing
        self.forward_input[..n].copy_from_slice(frame);
        self.forward_input[n..FFT_SIZE].fill(0.0);

        // 3. Forward RealFFT
        let _ = self.fft_forward.process_with_scratch(
            &mut self.forward_input,
            &mut self.forward_output,
            &mut self.forward_scratch,
        );

        // 4. Power spectrum in frequency domain: S[k] = |X[k]|^2
        for (inv_bin, fwd_bin) in self.inverse_input.iter_mut().zip(self.forward_output.iter()) {
            *inv_bin = Complex32 {
                re: fwd_bin.re * fwd_bin.re + fwd_bin.im * fwd_bin.im,
                im: 0.0,
            };
        }

        // 5. Inverse RealFFT -> unnormalized linear autocorrelation r_t(tau) * (2 * n)
        let _ = self.fft_inverse.process_with_scratch(
            &mut self.inverse_input,
            &mut self.inverse_output,
            &mut self.inverse_scratch,
        );

        // 6. Compute NSDF n'_t(tau) = (2 * r_t(tau)) / m_t(tau)
        // With IFFT scale factor L = 2n, the numerator 2 * r_t(tau) = ifft_output[tau] / n
        let inv_n = 1.0 / (n as f32);
        for tau in 0..max_tau {
            let autocorr_numerator = self.inverse_output[tau] * inv_n;
            let m_tau = self.sq_prefix[n - tau] + (total_energy - self.sq_prefix[tau]);
            self.nsdf_buffer[tau] = if m_tau > 1e-9 {
                (autocorr_numerator / m_tau).clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }

        // 7. Extract primary peak (r1) and secondary non-harmonic peak (r2)
        let (r1, r2) = self.find_primary_and_secondary_peaks(max_tau);
        let clarity = r1.clamp(0.0, 1.0);
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

impl Default for NsdfEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
