use crate::audio_processing::instruments::instrument::InstrumentAcousticProfile;
use crate::audio_processing::instruments::notes::{DampingProfile, NoteArticulation};

/// Classifies note articulation based on sounding duration, attack slope, effective rise time,
/// strike velocity, transient crest factor, legato state, and instrument acoustic profile.
pub fn classify_articulation(
    duration_sec: f32,
    is_legato: bool,
    attack_slope: f32,
    effective_rise_sec: f32,
    crest_factor: f32,
    velocity: u8,
    profile: &InstrumentAcousticProfile,
) -> NoteArticulation {
    if is_legato {
        NoteArticulation::Legato
    } else if (attack_slope >= (profile.rapid_rise_min_slope * 100.0)
        && effective_rise_sec <= profile.rapid_rise_max_sec)
        || (velocity >= 90 && crest_factor >= 15.0)
    {
        NoteArticulation::Marcato
    } else if duration_sec <= profile.staccato_max_duration_sec {
        NoteArticulation::Staccato
    } else if duration_sec >= profile.tenuto_min_duration_sec {
        NoteArticulation::Tenuto
    } else {
        NoteArticulation::Normal
    }
}

/// Detects whether a note strike was an aggressive / mashed keybed slam.
pub fn detect_mashed_key(
    crest_factor: f32,
    sub_thump_dbfs: f32,
    profile: &InstrumentAcousticProfile,
) -> bool {
    crest_factor >= profile.mashed_crest_threshold_db
        && sub_thump_dbfs >= profile.mashed_sub_thump_dbfs
}

/// Classifies note release damping profile (dry damped vs. pedal sustained vs. half pedal).
pub fn classify_damping(
    duration_sec: f32,
    profile: &InstrumentAcousticProfile,
) -> DampingProfile {
    if duration_sec >= profile.tenuto_min_duration_sec {
        DampingProfile::PedalSustained
    } else if duration_sec <= profile.staccato_max_duration_sec {
        DampingProfile::DryDamped
    } else {
        DampingProfile::HalfPedal
    }
}
