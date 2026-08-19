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
        let mut filter = BandPassFilter::new(30.0, 10000.0, 44100);
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
            &mut filter,
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
            &mut filter,
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
            &mut filter,
            &instrument,
            &mut mpm,
            ts + 20,
        );
        assert_eq!(extractor.state(), FeatureExtractorState::Idle);
        assert!(extractor.active_note().is_none());

        // Verify emitted (Note, timestamp)
        let (emitted_note, timestamp) = opt3.expect("Should have returned emitted note");
        assert_eq!(emitted_note.pitch, Pitch::A);
        assert_eq!(emitted_note.octave, Octave::O4);
        assert!(emitted_note.loudness_dbfs > -20.0);
        assert_eq!(timestamp, ts + 20);
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
        let mut filter = BandPassFilter::new(30.0, 10000.0, 44100);
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
            &mut filter,
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
            &mut filter,
            &instrument,
            &mut mpm,
            ts + 10,
        );

        // First note (C4) should have been emitted
        let (first_note, ts1) = opt2.expect("First note (C4) should be emitted");
        assert_eq!(first_note.pitch, Pitch::C);
        assert_eq!(first_note.octave, Octave::O4);
        assert_eq!(ts1, ts + 10);

        // Active note is now D4
        assert_eq!(extractor.active_note().unwrap().pitch, Pitch::D);
        assert_eq!(extractor.active_note().unwrap().octave, Octave::O4);

        // 3. Release Note D4 with silence
        let opt3 = extractor.processing_single_note(
            &silent_frame,
            &cfg,
            &mut filter,
            &instrument,
            &mut mpm,
            ts + 20,
        );
        assert_eq!(extractor.state(), FeatureExtractorState::Idle);

        let (second_note, ts2) = opt3.expect("Second note (D4) should be emitted");
        assert_eq!(second_note.pitch, Pitch::D);
        assert_eq!(second_note.octave, Octave::O4);
        assert_eq!(ts2, ts + 20);
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
        let rb = HeapRb::<(Note, u128)>::new(16);
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
        let note_opt = poly.process_polyphonic_path(&silent_frame, 44100.0, 1000, -30.0);
        assert!(note_opt.is_none());
    }
}
