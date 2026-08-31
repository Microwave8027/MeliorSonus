use crate::constants::FRAME_SIZE;
use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};
use std::f32::consts::PI;
use std::sync::Arc;

pub const SPECTRUM_BINS: usize = FRAME_SIZE / 2;

/// SIMD-accelerated zero-allocation Real Fast Fourier Transform analyzer using `realfft`.
/// Pre-allocates Hann window, complex spectrum vectors, and scratch buffers at startup.
pub struct RealFft {
    fft: Arc<dyn RealToComplex<f32>>,
    window: [f32; FRAME_SIZE],
    input_buf: Vec<f32>,
    output_buf: Vec<Complex32>,
    scratch_buf: Vec<Complex32>,
    pub magnitude: [f32; SPECTRUM_BINS],
}

impl RealFft {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FRAME_SIZE);

        let mut window = [0.0f32; FRAME_SIZE];
        for (i, w) in window.iter_mut().enumerate() {
            // Periodic Hann window
            *w = 0.5 * (1.0 - (2.0 * PI * (i as f32) / (FRAME_SIZE as f32)).cos());
        }

        let input_buf = fft.make_input_vec();
        let output_buf = fft.make_output_vec();
        let scratch_buf = fft.make_scratch_vec();

        Self {
            fft,
            window,
            input_buf,
            output_buf,
            scratch_buf,
            magnitude: [0.0; SPECTRUM_BINS],
        }
    }

    /// Computes windowed forward FFT using SIMD acceleration and stores the magnitude spectrum in `self.magnitude`.
    /// Guaranteed 0 heap allocations during invocation.
    #[inline(always)]
    pub fn compute_magnitude_spectrum(&mut self, frame: &[f32; FRAME_SIZE]) {
        // 1. Apply Hann window into pre-allocated input vector
        for i in 0..FRAME_SIZE {
            self.input_buf[i] = frame[i] * self.window[i];
        }

        // 2. Execute forward SIMD real-to-complex FFT
        let _ = self.fft.process_with_scratch(
            &mut self.input_buf,
            &mut self.output_buf,
            &mut self.scratch_buf,
        );

        // 3. Extract single-sided magnitude spectrum for first SPECTRUM_BINS
        for k in 0..SPECTRUM_BINS {
            let c = self.output_buf[k];
            self.magnitude[k] = (c.re * c.re + c.im * c.im).sqrt();
        }
    }
}

impl Default for RealFft {
    fn default() -> Self {
        Self::new()
    }
}
