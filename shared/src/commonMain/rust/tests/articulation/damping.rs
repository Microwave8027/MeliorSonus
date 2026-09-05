use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;

#[test]
fn test_damping_detection_piano_vs_violin() {
    let piano_profile = Instrument::Piano.acoustic_profile();

    let note1 = RecordNote::new_with_features(
        Pitch::A,
        Octave::O4,
        0,
        -15.0,
        1000,
        12.0,
        -60.0,
        1500.0,
        Some(0.95),
        None,
        None,
    );
    // 50ms duration (staccato <= 0.08) -> DryDamped
    let end1 = note1.into_end_note_with_profile(1050, false, &piano_profile);
    assert_eq!(end1.damping, DampingProfile::DryDamped);

    let note2 = RecordNote::new_with_features(
        Pitch::A,
        Octave::O4,
        0,
        -15.0,
        1000,
        12.0,
        -60.0,
        1500.0,
        Some(0.95),
        None,
        None,
    );
    // 1.8s duration (tenuto >= 0.6) -> PedalSustained
    let end2 = note2.into_end_note_with_profile(2800, false, &piano_profile);
    assert_eq!(end2.damping, DampingProfile::PedalSustained);
}
