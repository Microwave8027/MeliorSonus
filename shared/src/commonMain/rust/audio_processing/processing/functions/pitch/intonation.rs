use crate::audio_processing::instruments::instrument::InstrumentAcousticProfile;

/// Evaluates pitch intonation against instrument-specific tuning thresholds.
///
/// Returns `(is_flat, is_sharp)`.
pub fn evaluate_intonation(
    cents_offset: i8,
    profile: &InstrumentAcousticProfile,
) -> (bool, bool) {
    let is_flat = cents_offset < profile.flat_cents_threshold;
    let is_sharp = cents_offset > profile.sharp_cents_threshold;
    (is_flat, is_sharp)
}

/// Calculates pitch stability (cent range) across a note's sustained lifespan.
pub fn calculate_pitch_stability(min_cents: i8, max_cents: i8) -> u8 {
    (max_cents as i16 - min_cents as i16).unsigned_abs() as u8
}
