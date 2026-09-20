/*
 * PURE-RUST SLANEY LOG-MEL SPECTROGRAM FRONTEND FOR BYTEDANCE CRNN
 * Zero-allocation in audio loop, SIMD-accelerated via realfft.
 */

use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};
use std::f32::consts::PI;
use std::sync::Arc;

pub const CRNN_SAMPLE_RATE: f32 = 16000.0;
pub const CRNN_FFT_SIZE: usize = 2048;
pub const CRNN_MEL_BINS: usize = 229;
pub const CRNN_FFT_BINS: usize = CRNN_FFT_SIZE / 2 + 1; // 1025 bins
pub const CRNN_F_MIN: f32 = 30.0;
pub const CRNN_F_MAX: f32 = 8000.0;
pub const CRNN_LOG_CLAMP_MIN: f32 = 1e-10;

/// Pre-calculated sparse representation of a single triangular Mel filter.
#[derive(Clone, Debug)]
pub struct SlaneyMelFilter {
    pub start_bin: usize,
    pub weights: Vec<f32>,
}

/// SIMD-accelerated, zero-allocation Slaney Log-Mel Spectrogram extractor.
/// Computes the exact 229-bin Log-Mel representation required by the ByteDance CRNN.
pub struct SlaneyMelFrontend {
    fft_plan: Arc<dyn RealToComplex<f32>>,
    hann_window: [f32; CRNN_FFT_SIZE],
    filters: Vec<SlaneyMelFilter>,
    fft_input: Vec<f32>,
    fft_output: Vec<Complex32>,
    fft_scratch: Vec<Complex32>,
    power_spectrum: [f32; CRNN_FFT_BINS],
}

impl SlaneyMelFrontend {
    /// Constructs and pre-allocates all FFT plans, window coefficients, and Slaney Mel filterbank weights.
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft_plan = planner.plan_fft_forward(CRNN_FFT_SIZE);

        // 1. Precompute periodic Hann window
        let mut hann_window = [0.0f32; CRNN_FFT_SIZE];
        for (n, w) in hann_window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * PI * n as f32 / CRNN_FFT_SIZE as f32).cos());
        }

        // 2. Precompute Slaney 229-bin triangular filterbank
        let filters = Self::compute_slaney_filterbanks(
            CRNN_SAMPLE_RATE,
            CRNN_FFT_SIZE,
            CRNN_MEL_BINS,
            CRNN_F_MIN,
            CRNN_F_MAX,
        );

        let fft_input = fft_plan.make_input_vec();
        let fft_output = fft_plan.make_output_vec();
        let fft_scratch = fft_plan.make_scratch_vec();

        Self {
            fft_plan,
            hann_window,
            filters,
            fft_input,
            fft_output,
            fft_scratch,
            power_spectrum: [0.0f32; CRNN_FFT_BINS],
        }
    }

    /// [REAL-TIME SAFE: ZERO ALLOCATION]
    /// Computes 229 natural log-mel bins from a 2048-sample audio slice.
    #[inline]
    pub fn compute_log_mel_frame(
        &mut self,
        pcm_2048: &[f32; CRNN_FFT_SIZE],
        out_log_mel: &mut [f32; CRNN_MEL_BINS],
    ) {
        // 1. Apply Hann window into pre-allocated input vector
        for i in 0..CRNN_FFT_SIZE {
            self.fft_input[i] = pcm_2048[i] * self.hann_window[i];
        }

        // 2. Real-to-Complex forward FFT
        let _ = self.fft_plan.process_with_scratch(
            &mut self.fft_input,
            &mut self.fft_output,
            &mut self.fft_scratch,
        );

        // 3. Compute Power Spectrum: P[k] = Re^2 + Im^2
        for k in 0..CRNN_FFT_BINS {
            let c = self.fft_output[k];
            self.power_spectrum[k] = c.re * c.re + c.im * c.im;
        }

        // 4. Dot product with Slaney Triangular Filters + Clamped Natural Log: ln(max(x, 1e-5))
        for (m, filter) in self.filters.iter().enumerate() {
            let mut mel_energy = 0.0f32;
            let start = filter.start_bin;
            for (w_idx, &weight) in filter.weights.iter().enumerate() {
                mel_energy += self.power_spectrum[start + w_idx] * weight;
            }

            let clamped = if mel_energy < CRNN_LOG_CLAMP_MIN {
                CRNN_LOG_CLAMP_MIN
            } else {
                mel_energy
            };
            out_log_mel[m] = 10.0 * clamped.log10();
        }
    }

    pub fn filters(&self) -> &[SlaneyMelFilter] {
        &self.filters
    }

    pub fn hz_to_slaney_mel(f: f32) -> f32 {
        if f < 1000.0 {
            3.0 * f / 200.0
        } else {
            15.0 + 27.0 * (f / 1000.0).ln() / 6.4f32.ln()
        }
    }

    pub fn slaney_mel_to_hz(mel: f32) -> f32 {
        if mel < 15.0 {
            200.0 * mel / 3.0
        } else {
            1000.0 * 6.4f32.powf((mel - 15.0) / 27.0)
        }
    }

    /// Precomputes the Slaney area-normalized triangular Mel filterbank matrix.
    fn compute_slaney_filterbanks(
        sr: f32,
        n_fft: usize,
        n_mels: usize,
        f_min: f32,
        f_max: f32,
    ) -> Vec<SlaneyMelFilter> {
        let min_mel = Self::hz_to_slaney_mel(f_min);
        let max_mel = Self::hz_to_slaney_mel(f_max);

        // n_mels + 2 boundary points
        let num_points = n_mels + 2;
        let mut hz_points = Vec::with_capacity(num_points);
        for i in 0..num_points {
            let mel = min_mel + i as f32 * (max_mel - min_mel) / (n_mels as f32 + 1.0);
            hz_points.push(Self::slaney_mel_to_hz(mel));
        }

        let bin_step = sr / n_fft as f32; // 16000 / 2048 = 7.8125 Hz
        let mut filters = Vec::with_capacity(n_mels);

        for m in 0..n_mels {
            let left_hz = hz_points[m];
            let center_hz = hz_points[m + 1];
            let right_hz = hz_points[m + 2];

            let start_bin = (left_hz / bin_step).floor().max(0.0) as usize;
            let end_bin = ((right_hz / bin_step).ceil() as usize).min(CRNN_FFT_BINS - 1);

            let filter_area = 2.0 / (right_hz - left_hz); // Slaney area normalization: 2 / (f_right - f_left)
            let mut weights = Vec::with_capacity(end_bin.saturating_sub(start_bin) + 1);

            for k in start_bin..=end_bin {
                let freq = k as f32 * bin_step;
                let weight = if freq >= left_hz && freq <= center_hz {
                    if (center_hz - left_hz).abs() > 1e-7 {
                        (freq - left_hz) / (center_hz - left_hz)
                    } else {
                        0.0
                    }
                } else if freq > center_hz && freq <= right_hz {
                    if (right_hz - center_hz).abs() > 1e-7 {
                        (right_hz - freq) / (right_hz - center_hz)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                weights.push(weight * filter_area);
            }

            filters.push(SlaneyMelFilter { start_bin, weights });
        }

        filters
    }
}

impl Default for SlaneyMelFrontend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slaney_mel_scale_roundtrip() {
        let test_freqs = [30.0, 100.0, 440.0, 1000.0, 2000.0, 4186.0, 8000.0];
        for &f in &test_freqs {
            let mel = SlaneyMelFrontend::hz_to_slaney_mel(f);
            let reconstructed_hz = SlaneyMelFrontend::slaney_mel_to_hz(mel);
            assert!(
                (f - reconstructed_hz).abs() < 1e-3,
                "Failed roundtrip for {} Hz: got {}",
                f,
                reconstructed_hz
            );
        }
    }

    #[test]
    fn test_slaney_mel_filterbank_dimensions() {
        let frontend = SlaneyMelFrontend::new();
        assert_eq!(frontend.filters().len(), CRNN_MEL_BINS);

        for (m, filter) in frontend.filters().iter().enumerate() {
            assert!(
                filter.start_bin < CRNN_FFT_BINS,
                "Filter {} start_bin out of bounds: {}",
                m,
                filter.start_bin
            );
            assert!(
                !filter.weights.is_empty(),
                "Filter {} has empty weights",
                m
            );
            assert!(
                filter.start_bin + filter.weights.len() <= CRNN_FFT_BINS + 1,
                "Filter {} extends past FFT bins",
                m
            );
        }
    }

    #[test]
    fn test_silence_input_produces_clamped_log() {
        let mut frontend = SlaneyMelFrontend::new();
        let silent_pcm = [0.0f32; CRNN_FFT_SIZE];
        let mut log_mel = [0.0f32; CRNN_MEL_BINS];

        frontend.compute_log_mel_frame(&silent_pcm, &mut log_mel);

        let expected_min = 10.0 * CRNN_LOG_CLAMP_MIN.log10(); // 10 * log10(1e-10) = -100.0 dB
        for &val in &log_mel {
            assert!(
                (val - expected_min).abs() < 1e-4,
                "Expected clamped silence {}, got {}",
                expected_min,
                val
            );
        }
    }

    #[test]
    fn test_sine_wave_produces_localized_peak() {
        let mut frontend = SlaneyMelFrontend::new();
        let mut pcm = [0.0f32; CRNN_FFT_SIZE];

        // 440 Hz (A4) sine wave at 16 kHz
        for n in 0..CRNN_FFT_SIZE {
            pcm[n] = (2.0 * PI * 440.0 * n as f32 / CRNN_SAMPLE_RATE).sin();
        }

        let mut log_mel = [0.0f32; CRNN_MEL_BINS];
        frontend.compute_log_mel_frame(&pcm, &mut log_mel);

        // Find peak mel bin
        let mut max_val = f32::NEG_INFINITY;
        let mut max_bin = 0;
        for (i, &val) in log_mel.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_bin = i;
            }
        }

        // 440 Hz in Slaney Mel is 3 * 440 / 200 = 6.6 Mel
        // Total Mel span is from 30 Hz (0.45 Mel) to 8000 Hz (66.0 Mel)
        // Check that the peak bin is within the expected range around 440 Hz
        assert!(max_val > 0.0, "Sine peak should be well above silence floor");
        assert!(max_bin > 15 && max_bin < 40, "Peak bin for 440Hz was {}", max_bin);
    }
}
