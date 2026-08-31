use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::processing::functions::articulation::articulation_classifier::*;

#[test]
fn test_articulation_classifier_heuristics() {
    let piano_profile = Instrument::Piano.acoustic_profile();

    // 1. Legato
    assert_eq!(
        classify_articulation(0.5, true, 50.0, 0.005, 12.0, 70, &piano_profile),
        NoteArticulation::Legato
    );

    // 2. Marcato via high velocity & high crest factor
    assert_eq!(
        classify_articulation(0.4, false, 50.0, 0.005, 16.0, 95, &piano_profile),
        NoteArticulation::Marcato
    );

    // 3. Marcato via steep attack slope within rapid rise window
    assert_eq!(
        classify_articulation(0.4, false, 200.0, 0.010, 12.0, 70, &piano_profile),
        NoteArticulation::Marcato
    );

    // 4. Staccato
    assert_eq!(
        classify_articulation(0.15, false, 50.0, 0.005, 12.0, 70, &piano_profile),
        NoteArticulation::Staccato
    );

    // 5. Tenuto
    assert_eq!(
        classify_articulation(0.85, false, 50.0, 0.005, 12.0, 70, &piano_profile),
        NoteArticulation::Tenuto
    );

    // 6. Normal
    assert_eq!(
        classify_articulation(0.45, false, 50.0, 0.005, 12.0, 70, &piano_profile),
        NoteArticulation::Normal
    );

    // 7. Mashed key detection
    assert!(detect_mashed_key(17.0, -18.0, &piano_profile));
    assert!(!detect_mashed_key(12.0, -18.0, &piano_profile));
    assert!(!detect_mashed_key(17.0, -25.0, &piano_profile));

    // 8. Damping classification
    assert_eq!(classify_damping(0.15, &piano_profile), DampingProfile::DryDamped);
    assert_eq!(classify_damping(0.85, &piano_profile), DampingProfile::PedalSustained);
    assert_eq!(classify_damping(0.45, &piano_profile), DampingProfile::HalfPedal);
}

#[test]
fn test_record_note_delegated_articulation() {
    let piano_profile = Instrument::Piano.acoustic_profile();

    // Staccato note
    let rec = RecordNote::new(Pitch::C, Octave::O4, 0, -20.0, 1000, None);
    let end_staccato = rec.into_end_note_with_profile(1150, false, &piano_profile); // 150ms duration
    assert_eq!(end_staccato.articulation, NoteArticulation::Staccato);
    assert_eq!(end_staccato.damping, DampingProfile::DryDamped);

    // Tenuto note
    let rec2 = RecordNote::new(Pitch::D, Octave::O4, 0, -20.0, 1000, None);
    let end_tenuto = rec2.into_end_note_with_profile(1800, false, &piano_profile); // 800ms duration
    assert_eq!(end_tenuto.articulation, NoteArticulation::Tenuto);
    assert_eq!(end_tenuto.damping, DampingProfile::PedalSustained);
}
