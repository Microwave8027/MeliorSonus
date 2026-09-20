use crate::constants::FRAME_SIZE;
use super::real_fft::SPECTRUM_BINS;

/// Computes the Spectral Centroid (center of mass of the magnitude spectrum) in Hz.
///
/// Higher values indicate a brighter, more harmonic tone; lower values indicate a dark or muffled tone.
pub fn compute_spectral_centroid(magnitudes: &[f32; SPECTRUM_BINS], sample_rate: f32) -> f32 {
    let bin_resolution = sample_rate / (FRAME_SIZE as f32);
    let mut num = 0.0f32;
    let mut den = 0.0f32;

    for (k, &mag) in magnitudes.iter().enumerate() {
        let freq = (k as f32) * bin_resolution;
        num += freq * mag;
        den += mag;
    }

    if den > 1e-6 {
        num / den
    } else {
        0.0
    }
}
