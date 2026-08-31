use crate::constants::*;
use biquad::{Biquad, Coefficients, DirectForm2Transposed, Q_BUTTERWORTH_F32, ToHertz, Type};

/// Numerically stable 2nd-order Direct Form 2 Transposed Band-Pass Filter (Cascaded HPF + LPF).
/// Provides zero-allocation, SIMD/register-level sample filtering.
pub struct BandPassFilter {
    hpf: DirectForm2Transposed<f32>,
    lpf: DirectForm2Transposed<f32>,
}

impl BandPassFilter {
    pub fn new(high_pass_hz: f32, low_pass_hz: f32, sample_rate: u32) -> Self {
        let fs = (sample_rate as f32).hz();
        let nyquist = sample_rate as f32 * 0.5;

        let hp_val = high_pass_hz.clamp(10.0, nyquist - 100.0);
        let lp_val = low_pass_hz.clamp(hp_val + 50.0, nyquist - 50.0);

        let hp_cutoff = hp_val.hz();
        let lp_cutoff = lp_val.hz();

        let hp_coeffs =
            Coefficients::<f32>::from_params(Type::HighPass, fs, hp_cutoff, Q_BUTTERWORTH_F32)
                .unwrap_or_else(|_| {
                    Coefficients::<f32>::from_params(
                        Type::HighPass,
                        fs,
                        20.0.hz(),
                        Q_BUTTERWORTH_F32,
                    )
                    .expect("Valid default HPF coefficients")
                });

        let lp_coeffs =
            Coefficients::<f32>::from_params(Type::LowPass, fs, lp_cutoff, Q_BUTTERWORTH_F32)
                .unwrap_or_else(|_| {
                    Coefficients::<f32>::from_params(
                        Type::LowPass,
                        fs,
                        (nyquist - 100.0).hz(),
                        Q_BUTTERWORTH_F32,
                    )
                    .expect("Valid default LPF coefficients")
                });

        Self {
            hpf: DirectForm2Transposed::<f32>::new(hp_coeffs),
            lpf: DirectForm2Transposed::<f32>::new(lp_coeffs),
        }
    }

    #[inline(always)]
    pub fn process_sample(&mut self, sample: f32) -> f32 {
        let lp_out = self.lpf.run(sample);
        self.hpf.run(lp_out)
    }

    #[inline(always)]
    pub fn process_frames(&mut self, frames: &[f32; FRAME_SIZE]) -> [f32; FRAME_SIZE] {
        let mut output = [0.0f32; FRAME_SIZE];
        for i in 0..FRAME_SIZE {
            output[i] = self.process_sample(frames[i]);
        }
        output
    }

    #[inline(always)]
    pub fn process_hop(&mut self, hop: &[f32; HOP_SIZE]) -> [f32; HOP_SIZE] {
        let mut output = [0.0f32; HOP_SIZE];
        for i in 0..HOP_SIZE {
            output[i] = self.process_sample(hop[i]);
        }
        output
    }

    #[inline(always)]
    pub fn process_slice_in_place(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process_sample(*sample);
        }
    }
}
