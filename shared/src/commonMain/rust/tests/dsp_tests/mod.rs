#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::high_pass_filter::BandPassFilter;
    use crate::instruments::Instrument;
    use crate::monophonic_feature_extractor::*;
    use crate::mpm::MPM;
    use crate::notes::*;
    use crate::prelude::*;
    use ringbuf::HeapRb;
    use ringbuf::traits::Split;

    fn make_tone_frame(freq_hz: f32, amplitude: f32, sample_rate: u32) -> [f32; FRAME_SIZE] {
        let mut frame = [0.0f32; FRAME_SIZE];
        for (i, x) in frame.iter_mut().enumerate() {
            *x = amplitude
                * (2.0 * std::f32::consts::PI * freq_hz * (i as f32) / (sample_rate as f32)).sin();
        }
        frame
    }

    #[test]
    fn test_initial_state_idle() {
        let extractor = NoteFeatureExtractorImpl::new(-40.0);
        assert_eq!(extractor.state(), FeatureExtractorState::Idle);
        assert!(extractor.active_note().is_none());
    }

    #[test]
    fn test_note_lifecycle_attack_decay_release() {
        let mut extractor = NoteFeatureExtractorImpl::new(-40.0);

        let cfg = StreamConfig {
            channels: 1,
            sample_rate: 44100,
            buffer_size: cpal::BufferSize::Default,
        };
        let instrument = Instrument::Generic;
        let mut mpm = MPM::new(FRAME_SIZE / 2);

        let silent_frame = [0.0f32; FRAME_SIZE];
        let a4_attack = make_tone_frame(440.0, 0.6, 44100);
        let a4_decay = make_tone_frame(440.0, 0.3, 44100);

        let ts = 1000u128;

        // 1. Attack / Rise
        let opt1 = extractor.processing_single_note(
            &a4_attack,
            &cfg,
            &instrument,
            &mut mpm,
            ts,
        );
        assert!(opt1.is_none());
        assert_eq!(extractor.state(), FeatureExtractorState::Rise);
        assert!(extractor.active_note().is_some());
        assert_eq!(extractor.active_note().unwrap().pitch, Pitch::A);
        assert_eq!(extractor.active_note().unwrap().octave, Octave::O4);

        // 2. Peak passed -> Decay
        let opt2 = extractor.processing_single_note(
            &a4_decay,
            &cfg,
            &instrument,
            &mut mpm,
            ts + 10,
        );
        assert!(opt2.is_none());
        assert_eq!(extractor.state(), FeatureExtractorState::Decay);

        // 3. Silence -> Release to Idle
        let opt3 = extractor.processing_single_note(
            &silent_frame,
            &cfg,
            &instrument,
            &mut mpm,
            ts + 20,
        );
        assert_eq!(extractor.state(), FeatureExtractorState::Idle);
        assert!(extractor.active_note().is_none());

        // Verify emitted Note
        let emitted_note = opt3.expect("Should have returned emitted note");
        assert_eq!(emitted_note.pitch, Pitch::A);
        assert_eq!(emitted_note.octave, Octave::O4);
        assert!(emitted_note.loudness_dbfs > -20.0);
        assert_eq!(emitted_note.note_striked, ts);
        assert!(emitted_note.note_duration >= 0.0);
        assert!(emitted_note.rise_duration >= 0.0);
    }

    #[test]
    fn test_legato_transition() {
        let mut extractor = NoteFeatureExtractorImpl::new(-40.0);

        let cfg = StreamConfig {
            channels: 1,
            sample_rate: 44100,
            buffer_size: cpal::BufferSize::Default,
        };
        let instrument = Instrument::Generic;
        let mut mpm = MPM::new(FRAME_SIZE / 2);

        let c4_frame = make_tone_frame(261.63, 0.5, 44100);
        let d4_frame = make_tone_frame(293.66, 0.5, 44100);
        let silent_frame = [0.0f32; FRAME_SIZE];

        let ts = 1000u128;

        // 1. Play Note C4
        let opt1 = extractor.processing_single_note(
            &c4_frame,
            &cfg,
            &instrument,
            &mut mpm,
            ts,
        );
        assert!(opt1.is_none());
        assert_eq!(extractor.state(), FeatureExtractorState::Rise);
        assert_eq!(extractor.active_note().unwrap().pitch, Pitch::C);
        assert_eq!(extractor.active_note().unwrap().octave, Octave::O4);

        // 2. Legato transition directly into Note D4 (without silence)
        let opt2 = extractor.processing_single_note(
            &d4_frame,
            &cfg,
            &instrument,
            &mut mpm,
            ts + 10,
        );

        // First note (C4) should have been emitted
        let first_note = opt2.expect("First note (C4) should be emitted");
        assert_eq!(first_note.pitch, Pitch::C);
        assert_eq!(first_note.octave, Octave::O4);
        assert_eq!(first_note.note_striked, ts);

        // Active note is now D4
        assert_eq!(extractor.active_note().unwrap().pitch, Pitch::D);
        assert_eq!(extractor.active_note().unwrap().octave, Octave::O4);

        // 3. Release Note D4 with silence
        let opt3 = extractor.processing_single_note(
            &silent_frame,
            &cfg,
            &instrument,
            &mut mpm,
            ts + 20,
        );
        assert_eq!(extractor.state(), FeatureExtractorState::Idle);

        let second_note = opt3.expect("Second note (D4) should be emitted");
        assert_eq!(second_note.pitch, Pitch::D);
        assert_eq!(second_note.octave, Octave::O4);
        assert_eq!(second_note.note_striked, ts + 10);
    }

    #[test]
    fn test_frequency_to_note_boundaries() {
        // A4 = 440 Hz -> (Pitch::A, Octave::O4, 0)
        let note = get_note(440.0).expect("A4 should resolve");
        assert_eq!(note.0, Pitch::A);
        assert_eq!(note.1, Octave::O4);
        assert_eq!(note.2, 0);

        // Middle C (C4) ~ 261.63 Hz
        let c4 = get_note(261.63).expect("C4 should resolve");
        assert_eq!(c4.0, Pitch::C);
        assert_eq!(c4.1, Octave::O4);

        // Invalid frequencies
        assert!(get_note(0.0).is_none());
        assert!(get_note(-100.0).is_none());
        assert!(get_note(f32::NAN).is_none());
        assert!(get_note(f32::INFINITY).is_none());
    }

    #[test]
    fn test_nsdf_evaluator_monophonic_clarity() {
        use crate::nsdf::NsdfEvaluator;

        let mut nsdf = NsdfEvaluator::new();

        // 1. Pure A4 tone (440 Hz) at 44.1kHz -> High monophonic clarity
        let a4_frame = make_tone_frame(440.0, 0.8, 44100);
        let (clarity, ratio) = nsdf.evaluate_frame(&a4_frame);
        assert!(
            clarity >= 0.80,
            "Pure tone should yield high clarity >= 0.80, got {}",
            clarity
        );
        assert!(
            ratio <= 0.60,
            "Single tone secondary ratio should be low <= 0.60, got {}",
            ratio
        );

        // 2. Silent frame -> Low clarity
        let silent_frame = [0.0f32; FRAME_SIZE];
        let (silent_clarity, _) = nsdf.evaluate_frame(&silent_frame);
        assert_eq!(silent_clarity, 0.0);
    }

    #[test]
    fn test_dsp_feature_extractor_dynamic_mode_switching() {
        let rb = HeapRb::<Note>::new(16);
        let (prod, _cons) = rb.split();

        let single = NoteFeatureExtractorImpl::new(-40.0);
        let poly = PolyphonicFeatureExtractorImpl::new();
        let mut extractor = dsp_feature_extractor::new(prod, single, poly, -40.0);

        let cfg = StreamConfig {
            channels: 1,
            sample_rate: 44100,
            buffer_size: cpal::BufferSize::Default,
        };
        let mut filter = BandPassFilter::new(30.0, 10000.0, 44100);
        let instrument = Instrument::Piano;
        let mut mpm = MPM::new(FRAME_SIZE / 2);

        let silent_frame = [0.0f32; FRAME_SIZE];
        let a4_frame = make_tone_frame(440.0, 0.8, 44100);

        // 1. Silence
        extractor.dsp_callback(CallBackParameters {
            buffer: &silent_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
        assert_eq!(extractor.mode(), PolyphonyMode::Silence);

        // 2. Pure single A4 note -> Fast path (Monophonic MPM)
        extractor.dsp_callback(CallBackParameters {
            buffer: &a4_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
        assert_eq!(extractor.mode(), PolyphonyMode::SingleNoteFastPath);
    }

    #[test]
    fn test_polyphonic_voice_state_tracking() {
        let mut poly = PolyphonicFeatureExtractorImpl::new();
        assert_eq!(poly.poly_note_states[0], PolyphonicNoteState::Idle);

        let silent_frame = [0.0f32; FRAME_SIZE];
        let notes = poly.process_polyphonic_path(&silent_frame, 44100.0, 1000, -30.0);
        assert!(notes.is_empty());
    }

    #[test]
    fn test_update_note_striked() {
        let mut note = Note {
            pitch: Pitch::C,
            octave: Octave::O4,
            tonality_offset: 0,
            loudness_dbfs: -12.0,
            rise_duration: 0.1,
            note_duration: 0.5,
            note_striked: 0,
        };
        note.update_note_striked(123456789);
        assert_eq!(note.note_striked, 123456789);

        update_note_striked(&mut note, 987654321);
        assert_eq!(note.note_striked, 987654321);
    }

    #[test]
    fn test_active_note_migration_mono_to_poly() {
        let mut mono = NoteFeatureExtractorImpl::new(-40.0);
        let mut poly = PolyphonicFeatureExtractorImpl::new();

        let cfg = StreamConfig {
            channels: 1,
            sample_rate: 44100,
            buffer_size: cpal::BufferSize::Default,
        };
        let instrument = Instrument::Generic;
        let mut mpm = MPM::new(FRAME_SIZE / 2);
        let a4_frame = make_tone_frame(440.0, 0.6, 44100);

        // Start note in mono extractor
        mono.processing_single_note(&a4_frame, &cfg, &instrument, &mut mpm, 1000);
        assert!(mono.active_note().is_some());

        // Migrate to poly
        let (active, state) = mono.take_active_note().expect("Should have active note");
        assert!(mono.active_note().is_none());
        assert_eq!(active.note_striked, 1000);
        poly.adopt_note(active, state);

        assert!(poly.poly_active_notes[0].is_some());
        assert_eq!(poly.poly_active_notes[0].as_ref().unwrap().pitch, Pitch::A);
        assert_eq!(poly.poly_active_notes[0].as_ref().unwrap().octave, Octave::O4);
        assert_eq!(poly.poly_active_notes[0].as_ref().unwrap().note_striked, 1000);
    }

    #[test]
    fn test_active_note_migration_poly_to_mono() {
        let mut mono = NoteFeatureExtractorImpl::new(-40.0);
        let mut poly = PolyphonicFeatureExtractorImpl::new();

        // Put 2 notes into poly with their onset timestamps
        poly.poly_active_notes[0] = Some(RecordNote::new(Pitch::C, Octave::O4, 0, -20.0, 1000));
        poly.poly_note_states[0] = PolyphonicNoteState::Decay;
        poly.poly_active_notes[1] = Some(RecordNote::new(Pitch::E, Octave::O4, 0, -10.0, 1200)); // louder
        poly.poly_note_states[1] = PolyphonicNoteState::Rise;

        let mut finalized_notes = Vec::new();
        let (primary, state) = poly
            .take_primary_active_note_and_finalize_rest(|n| finalized_notes.push(n))
            .expect("Should return primary note");

        // Louder note (E4) should be chosen as primary, preserving its onset timestamp
        assert_eq!(primary.pitch, Pitch::E);
        assert_eq!(primary.note_striked, 1200);
        assert_eq!(state, PolyphonicNoteState::Rise);

        // Trailing note (C4) should be finalized with its original onset timestamp
        assert_eq!(finalized_notes.len(), 1);
        assert_eq!(finalized_notes[0].pitch, Pitch::C);
        assert_eq!(finalized_notes[0].note_striked, 1000);

        // Mono adopts primary
        mono.adopt_note(primary, state);
        assert!(mono.active_note().is_some());
        assert_eq!(mono.active_note().unwrap().pitch, Pitch::E);
        assert_eq!(mono.active_note().unwrap().note_striked, 1200);
        assert_eq!(mono.state(), FeatureExtractorState::Rise);
    }

    #[test]
    fn test_polyphonic_multi_note_simultaneous_finalization() {
        let mut poly = PolyphonicFeatureExtractorImpl::new();

        // Simulate 3 notes actively sounding in polyphonic extractor with onset timestamp 1000
        poly.poly_active_notes[0] = Some(RecordNote::new(Pitch::C, Octave::O4, 0, -15.0, 1000));
        poly.poly_note_states[0] = PolyphonicNoteState::Decay;
        poly.poly_active_notes[1] = Some(RecordNote::new(Pitch::E, Octave::O4, 0, -14.0, 1000));
        poly.poly_note_states[1] = PolyphonicNoteState::Decay;
        poly.poly_active_notes[2] = Some(RecordNote::new(Pitch::G, Octave::O4, 0, -16.0, 1000));
        poly.poly_note_states[2] = PolyphonicNoteState::Decay;

        // Process a silent / inactive frame where probabilities drop below threshold (e.g. all 0.0) at timestamp 5000
        let silent_frame = [0.0f32; FRAME_SIZE];
        let finalized = poly.process_polyphonic_path(&silent_frame, 44100.0, 5000, -50.0);

        // All 3 notes should be finalized simultaneously without being overwritten
        assert_eq!(finalized.len(), 3, "All 3 chord notes must be finalized in the same frame");
        let pitches: Vec<Pitch> = finalized.iter().map(|n| n.pitch).collect();
        assert!(pitches.contains(&Pitch::C));
        assert!(pitches.contains(&Pitch::E));
        assert!(pitches.contains(&Pitch::G));

        for note in &finalized {
            // Note striked preserves original onset timestamp 1000
            assert_eq!(note.note_striked, 1000);
        }
    }

    #[test]
    fn test_decay_restrike_detection_and_last_dbfs_update() {
        let mut state = NoteEnvelopeState::Decay;
        let mut active = Some(RecordNote::new(Pitch::A, Octave::O4, 0, -10.0, 1000));
        active.as_mut().unwrap().last_dbfs = -20.0;

        // 1. Same pitch, but slight decay from -20.0 -> -22.0 (no restrike)
        let note1 = track_note_state(
            &mut state,
            &mut active,
            Some((Pitch::A, Octave::O4, 0)),
            -22.0,
            1020,
            |_| {},
            |_, _| {},
            |_| {},
        );
        assert!(note1.is_none());
        assert_eq!(state, NoteEnvelopeState::Decay);
        assert_eq!(active.as_ref().unwrap().last_dbfs, -22.0);

        // 2. Restrike at 1050: amplitude jumps from -22.0 -> -15.0 (> 3dB jump)
        let note2 = track_note_state(
            &mut state,
            &mut active,
            Some((Pitch::A, Octave::O4, 0)),
            -15.0,
            1050,
            |_| {},
            |_, _| {},
            |_| {},
        );
        // The previous note (onset at 1000) should be finalized
        let finalized = note2.expect("Restrike must finalize the decayed note");
        assert_eq!(finalized.pitch, Pitch::A);
        assert_eq!(finalized.note_striked, 1000);

        // A new note is started in Rise state with onset timestamp 1050
        assert_eq!(state, NoteEnvelopeState::Rise);
        assert!(active.is_some());
        assert_eq!(active.as_ref().unwrap().note_striked, 1050);
        assert_eq!(active.as_ref().unwrap().last_dbfs, -15.0);
    }

    #[test]
    fn test_harmonic_sieve_masks_ghost_but_preserves_chord_notes() {
        use crate::harmonic_sieve_mask::HarmonicSieveMasker;

        let mut sieve = HarmonicSieveMasker::new(0.0002, 6);
        let mut raw_probs = [0.0f32; PITCH_BINS];
        let mut sieved_probs = [0.0f32; PITCH_BINS];

        // Bin 36 is A3 (MIDI 57, 220 Hz), Bin 48 is A4 (MIDI 69, 440 Hz)
        raw_probs[36] = 0.90; // Loud A3 fundamental

        // 1. Ghost overtone: A4 has low leakage activation 0.35 -> should be masked to 0.0
        raw_probs[48] = 0.35;
        sieve.apply_sieve(&raw_probs, 44100.0, &mut sieved_probs);
        assert_eq!(sieved_probs[36], 0.90);
        assert_eq!(
            sieved_probs[48], 0.0,
            "Low-confidence overtone ghost must be masked to 0.0"
        );

        // 2. Genuine chord octave: A4 is intentionally played with high confidence 0.75 -> must be preserved!
        raw_probs[48] = 0.75;
        sieve.apply_sieve(&raw_probs, 44100.0, &mut sieved_probs);
        assert_eq!(sieved_probs[36], 0.90);
        assert_eq!(
            sieved_probs[48], 0.75,
            "High-confidence genuine chord note must NOT be zeroed by the harmonic sieve"
        );
    }
}
