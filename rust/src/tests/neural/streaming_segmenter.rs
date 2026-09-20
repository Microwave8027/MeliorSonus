use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::crnn::BasicPitchOutput;
use crate::audio_processing::neural::{SegmentedNoteEvent, StreamingNoteSegmenter};

#[test]
fn test_streaming_segmenter_monophonic_lifecycle() {
    let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 48000);
    let profile = Instrument::Piano.acoustic_profile();

    // 1. Onset frame: 440 Hz (A4), clarity = 0.95, dBFS = -12.0
    let event1 = segmenter.process_mpm_frame(
        (440.0, 0.95, -12.0),
        None,
        1000,
        12.0,
        -60.0,
        1500.0,
        &profile,
    );
    assert_eq!(event1.len(), 1);
    match &event1[0] {
        SegmentedNoteEvent::Start(s) => {
            assert_eq!(s.pitch, Pitch::A);
            assert_eq!(s.octave, Octave::O4);
            assert_eq!(s.note_striked, 1000);
        }
        _ => panic!("Expected StartNote"),
    }

    // 2. Sustain frame: 440 Hz (A4), clarity = 0.92, dBFS = -15.0 -> Returns empty
    let event2 = segmenter.process_mpm_frame(
        (440.0, 0.92, -15.0),
        None,
        1050,
        8.0,
        -60.0,
        1400.0,
        &profile,
    );
    assert!(event2.is_empty());

    // 3. Restrike via HFC onset at confirmed ts = 1090 (detected at frame ts = 1100, 90ms >= 80ms restrike debounce) -> Emits EndNote (old) and StartNote (new)
    let event3 = segmenter.process_mpm_frame(
        (440.0, 0.95, -10.0),
        Some(1090), // Confirmed candidate onset timestamp!
        1100,
        14.0,
        -60.0,
        2000.0,
        &profile,
    );
    assert_eq!(event3.len(), 2);
    match &event3[0] {
        SegmentedNoteEvent::End(e) => {
            assert_eq!(e.pitch, Pitch::A);
            assert_eq!(e.note_striked, 1000);
            assert!((e.note_duration - 0.090).abs() < 0.001);
        }
        _ => panic!("Expected EndNote on restrike"),
    }
    match &event3[1] {
        SegmentedNoteEvent::Start(s) => {
            assert_eq!(s.pitch, Pitch::A);
            assert_eq!(s.octave, Octave::O4);
            assert_eq!(s.note_striked, 1090);
        }
        _ => panic!("Expected StartNote on restrike"),
    }

    // 4. Release frame (low clarity) at ts = 1200
    let event4 = segmenter.process_mpm_frame(
        (0.0, 0.1, -50.0),
        None,
        1200,
        4.0,
        -60.0,
        500.0,
        &profile,
    );
    assert_eq!(event4.len(), 1);
    match &event4[0] {
        SegmentedNoteEvent::End(e) => {
            assert_eq!(e.pitch, Pitch::A);
            assert_eq!(e.octave, Octave::O4);
            assert_eq!(e.note_striked, 1090);
            assert!((e.note_duration - 0.110).abs() < 0.001);
        }
        _ => panic!("Expected EndNote on release"),
    }
}

#[test]
fn test_streaming_segmenter_polyphonic_multi_note() {
    let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 48000);
    let profile = Instrument::Piano.acoustic_profile();

    let mut out = BasicPitchOutput {
        frames: [0.0; 88],
        onsets: [0.0; 88],
        energy_dbfs: -15.0,
        ..Default::default()
    };

    // Strike C4 (MIDI 60 - 21 = index 39) and E4 (MIDI 64 - 21 = index 43)
    out.frames[39] = 0.85;
    out.onsets[39] = 0.90;
    out.frames[43] = 0.85;
    out.onsets[43] = 0.90;

    let dummy_frame = [0.0f32; 1024];
    let events = segmenter.process_crnn_frame(&out, 1000, 12.0, -60.0, 1200.0, &profile, &dummy_frame);
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|e| matches!(e, SegmentedNoteEvent::Start(s) if s.pitch == Pitch::C && s.octave == Octave::O4)));
    assert!(events.iter().any(|e| matches!(e, SegmentedNoteEvent::Start(s) if s.pitch == Pitch::E && s.octave == Octave::O4)));

    // Release C4, sustain E4 at ts = 1100
    out.frames[39] = 0.1;
    out.onsets[39] = 0.0;
    out.onsets[43] = 0.0;

    let events2 = segmenter.process_crnn_frame(&out, 1100, 8.0, -60.0, 1100.0, &profile, &dummy_frame);
    assert_eq!(events2.len(), 1);
    match &events2[0] {
        SegmentedNoteEvent::End(e) => {
            assert_eq!(e.pitch, Pitch::C);
            assert_eq!(e.octave, Octave::O4);
            assert_eq!(e.note_striked, 1000);
        }
        _ => panic!("Expected EndNote for C4"),
    }

    // Finalize all active notes at ts = 1200
    let finalized = segmenter.finalize_all(1200, &profile);
    assert_eq!(finalized.len(), 1);
    assert_eq!(finalized[0].pitch, Pitch::E);
    assert_eq!(finalized[0].octave, Octave::O4);
}
