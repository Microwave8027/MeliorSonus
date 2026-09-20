use crate::constants::FRAME_SIZE;

/// Computes the instantaneous crest factor (Peak-to-RMS ratio in dB) of an audio frame.
///
/// High crest factor (> 16 dB) indicates a sharp, punchy, or aggressively struck transient.
#[inline]
pub fn compute_crest_factor(frame: &[f32; FRAME_SIZE]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }

    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f32;

    for &sample in frame {
        let abs = sample.abs();
        if abs > peak {
            peak = abs;
        }
        sum_sq += sample * sample;
    }

    let rms = (sum_sq / frame.len() as f32).sqrt();
    if rms < 1e-6 || peak < 1e-6 {
        return 0.0;
    }

    20.0 * (peak / rms).log10()
}
