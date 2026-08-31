use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::{SegmentedNoteEvent, StreamingNoteSegmenter};

#[test]
fn test_hybrid_mode_state_transfer() {
    let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4);
    let profile = Instrument::Piano.acoustic_profile();

    // 1. Monophonic note (A4) active in MPM
    let event = segmenter.process_mpm_frame(
        (440.0, 0.95, -12.0),
        None,
        1000,
        12.0,
        -60.0,
        1500.0,
        &profile,
    );
    assert_eq!(event.len(), 1);
    assert!(matches!(event[0], SegmentedNoteEvent::Start(_)));
    assert!(segmenter.active_mono_note().is_some());

    // 2. Transfer mono to poly
    segmenter.transfer_mono_to_poly();
    assert!(segmenter.active_mono_note().is_none());
    assert!(segmenter.active_notes()[48].is_some()); // A4 index = 48

    // 3. Transfer poly back to mono for A4
    let finalized =
        segmenter.transfer_poly_to_mono(Some((Pitch::A, Octave::O4)), 1100, &profile);
    assert!(finalized.is_empty());
    assert!(segmenter.active_mono_note().is_some());
    assert!(segmenter.active_notes()[48].is_none());
}
