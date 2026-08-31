use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::utils::feature_extractor_state::{NoteEnvelopeState, track_note_state};

#[test]
fn test_note_state_machine_transitions() {
    let mut state = NoteEnvelopeState::Idle;
    let mut active_note: Option<RecordNote> = None;
    let profile = Instrument::Piano.acoustic_profile();

    // Transition: Idle -> Rise (A4 detected)
    let res1 = track_note_state(
        &mut state,
        &mut active_note,
        Some((Pitch::A, Octave::O4, 0)),
        -10.0,
        1000,
        12.0,
        -60.0,
        1500.0,
        Some(0.95),
        &profile,
        |_| {},
        |_, _| {},
        |_| {},
    );
    assert_eq!(state, NoteEnvelopeState::Rise);
    assert_eq!(res1.len(), 1);
    assert!(res1[0].is_start());

    // Transition: Rise -> Decay (energy dropped slightly)
    let res2 = track_note_state(
        &mut state,
        &mut active_note,
        Some((Pitch::A, Octave::O4, 0)),
        -15.0,
        1050,
        8.0,
        -60.0,
        1400.0,
        Some(0.92),
        &profile,
        |_| {},
        |_, _| {},
        |_| {},
    );
    assert_eq!(state, NoteEnvelopeState::Decay);
    assert_eq!(res2.len(), 0);

    // Transition: Decay -> Rise (re-attack / crescendo: triggers EndNote for old note and StartNote for new note)
    let res3 = track_note_state(
        &mut state,
        &mut active_note,
        Some((Pitch::A, Octave::O4, 0)),
        -8.0,
        1100,
        14.0,
        -60.0,
        2000.0,
        Some(0.95),
        &profile,
        |_| {},
        |_, _| {},
        |_| {},
    );
    assert_eq!(state, NoteEnvelopeState::Rise);
    assert_eq!(res3.len(), 2);

    // Transition: Rise -> Idle (silence gate)
    let res4 = track_note_state(
        &mut state,
        &mut active_note,
        None,
        -50.0,
        1200,
        4.0,
        -60.0,
        500.0,
        None,
        &profile,
        |_| {},
        |_, _| {},
        |_| {},
    );
    assert_eq!(state, NoteEnvelopeState::Idle);
    assert_eq!(res4.len(), 1);
    assert!(res4[0].is_end());
}
