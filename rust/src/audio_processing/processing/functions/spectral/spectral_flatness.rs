use super::real_fft::SPECTRUM_BINS;

/// Computes the Spectral Flatness Measure (Wiener Entropy) of a magnitude spectrum.
///
/// Output is bounded in `[0.0, 1.0]`:
/// - Near 0.0 indicates a resonant, pure tonal / harmonic structure.
/// - Near 1.0 indicates a noisy, inharmonic, or flat/dull noise spectrum.
pub fn compute_spectral_flatness(magnitudes: &[f32; SPECTRUM_BINS]) -> f32 {
    let mut sum_power = 0.0f32;
    let mut sum_log_power = 0.0f32;
    let n = magnitudes.len() as f32;

    for &mag in magnitudes {
        let power = (mag * mag).max(1e-12);
        sum_power += power;
        sum_log_power += power.ln();
    }

    let arithmetic_mean = sum_power / n;
    let geometric_mean = (sum_log_power / n).exp();

    if arithmetic_mean > 1e-9 {
        (geometric_mean / arithmetic_mean).clamp(0.0, 1.0)
    } else {
        0.0
    }
}
