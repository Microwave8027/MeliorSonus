use crate::constants::*;
use std::f32::consts::PI;

pub struct BandPassFilter {
    hbf: BiquadFilter,
    lbf: BiquadFilter,
}

enum FilterType {
    HighPass,
    LowPass,
}
pub struct BiquadFilter {
    filter_type: FilterType,
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    s1: f32,
    s2: f32,
}

impl BandPassFilter {
    pub fn new(high_pass_filter: f32, low_pass_filter: f32, sample_rate: u32) -> Self {
        let high = BiquadFilter {
            filter_type: FilterType::HighPass,
            a1: 0.0,
            a2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            s1: 0.0,
            s2: 0.0,
        };
        let low = BiquadFilter {
            filter_type: FilterType::LowPass,
            a1: 0.0,
            a2: 0.0,
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            s1: 0.0,
            s2: 0.0,
        };
        let high = Self::update_coefficients(high, sample_rate, high_pass_filter);
        let low = Self::update_coefficients(low, sample_rate, low_pass_filter);
        BandPassFilter {
            hbf: high,
            lbf: low,
        }
    }

    fn update_coefficients(
        filter_type: BiquadFilter,
        sample_rate: u32,
        limit: f32,
    ) -> BiquadFilter {
        let cut_off: f32 = limit.clamp(10.0, (sample_rate as f32 * 0.5) - 100.0);
        let omega: f32 = 2.0 * PI * cut_off / (sample_rate as f32);
        let q: f32 = std::f32::consts::FRAC_1_SQRT_2;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);

        let (b0, b1, b2, a1, a2) = match filter_type.filter_type {
            FilterType::HighPass => {
                let b0 = (1.0 + cos_w) * 0.5;
                let b1 = -(1.0 + cos_w);
                let b2 = (1.0 + cos_w) * 0.5;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a1, a2)
            }
            FilterType::LowPass => {
                let b0 = (1.0 - cos_w) * 0.5;
                let b1 = 1.0 - cos_w;
                let b2 = (1.0 - cos_w) * 0.5;
                let a1 = -2.0 * cos_w;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a1, a2)
            }
        };

        let inv = 1.0 / (1.0 + alpha);
        BiquadFilter {
            filter_type: filter_type.filter_type,
            a1: a1 * inv,
            a2: a2 * inv,
            b0: b0 * inv,
            b1: b1 * inv,
            b2: b2 * inv,
            s1: 0.0,
            s2: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_frames(&mut self, frames: &[f32; FRAME_SIZE]) -> [f32; FRAME_SIZE] {
        let mut output = [0.0; FRAME_SIZE];
        for i in 0..frames.len() {
            let y = self.lbf.b0 * frames[i] + self.lbf.s1;
            self.lbf.s1 = self.lbf.b1 * frames[i] - self.lbf.a1 * y + self.lbf.s2;
            self.lbf.s2 = self.lbf.b2 * frames[i] - self.lbf.a2 * y;

            let z = self.hbf.b0 * y + self.hbf.s1;
            self.hbf.s1 = self.hbf.b1 * y - self.hbf.a1 * z + self.hbf.s2;
            self.hbf.s2 = self.hbf.b2 * y - self.hbf.a2 * z;
            output[i] = z;
        }
        output
    }
}
