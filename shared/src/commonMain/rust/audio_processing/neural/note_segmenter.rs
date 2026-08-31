use crate::audio_processing::instruments::instrument::InstrumentAcousticProfile;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::crnn::{BasicPitchOutput, MIDI_OFFSET, NUM_PITCH_BINS};
use arrayvec::ArrayVec;

pub const MAX_POLYPHONIC_NOTES: usize = NUM_PITCH_BINS * 2;

#[derive(Clone, Debug, PartialEq)]
pub enum SegmentedNoteEvent {
    Start(StartNote),
    End(EndNote),
}

pub struct StreamingNoteSegmenter {
    active_notes: [Option<RecordNote>; NUM_PITCH_BINS],
    active_mono_note: Option<RecordNote>,
    onset_threshold: f32,
    frame_threshold: f32,
    min_note_duration_sec: f32,
}

impl StreamingNoteSegmenter {
    pub fn new(onset_threshold: f32, frame_threshold: f32) -> Self {
        Self {
            active_notes: core::array::from_fn(|_| None),
            active_mono_note: None,
            onset_threshold,
            frame_threshold,
            min_note_duration_sec: 0.020,
        }
    }

    pub fn frame_threshold(&self) -> f32 {
        self.frame_threshold
    }

    pub fn active_mono_note(&self) -> Option<&RecordNote> {
        self.active_mono_note.as_ref()
    }

    pub fn active_notes(&self) -> &[Option<RecordNote>; NUM_PITCH_BINS] {
        &self.active_notes
    }

    /// Converts a frequency in Hz into a MIDI pitch index (0..NUM_PITCH_BINS).
    pub fn freq_to_midi_index(freq: f32) -> Option<usize> {
        if freq < 27.5 || freq > 4186.0 || freq.is_nan() || freq.is_infinite() {
            return None;
        }
        let midi = (69.0 + 12.0 * (freq / 440.0).log2()).round() as usize;
        if midi >= MIDI_OFFSET && midi < MIDI_OFFSET + NUM_PITCH_BINS {
            Some(midi - MIDI_OFFSET)
        } else {
            None
        }
    }

    /// Converts a (Pitch, Octave) into a MIDI pitch index (0..NUM_PITCH_BINS).
    pub fn note_to_midi_index(pitch: Pitch, octave: Octave) -> Option<usize> {
        let pitch_class = match pitch {
            Pitch::C => 0,
            Pitch::CsDf => 1,
            Pitch::D => 2,
            Pitch::DsEf => 3,
            Pitch::E => 4,
            Pitch::F => 5,
            Pitch::FsGf => 6,
            Pitch::G => 7,
            Pitch::GsAf => 8,
            Pitch::A => 9,
            Pitch::AsBf => 10,
            Pitch::B => 11,
            Pitch::None => return None,
        };

        let octave_num: i8 = match octave {
            Octave::O_1 => -1,
            Octave::O0 => 0,
            Octave::O1 => 1,
            Octave::O2 => 2,
            Octave::O3 => 3,
            Octave::O4 => 4,
            Octave::O5 => 5,
            Octave::O6 => 6,
            Octave::O7 => 7,
            Octave::O8 => 8,
            Octave::O9 => 9,
            Octave::O10 | Octave::OutOfRange => return None,
        };

        let midi_number = (octave_num + 1) * 12 + pitch_class;
        if midi_number >= (MIDI_OFFSET as i8)
            && midi_number < ((MIDI_OFFSET + NUM_PITCH_BINS) as i8)
        {
            Some((midi_number as usize) - MIDI_OFFSET)
        } else {
            None
        }
    }

    /// Converts a MIDI pitch index (0..NUM_PITCH_BINS) back into (Pitch, Octave).
    pub fn midi_index_to_note(idx: usize) -> (Pitch, Octave) {
        let midi = (idx + MIDI_OFFSET) as i32;
        let pitch_class = (midi % 12) as u8;
        let octave_num = (midi / 12) - 1;

        let pitch = match pitch_class {
            0 => Pitch::C,
            1 => Pitch::CsDf,
            2 => Pitch::D,
            3 => Pitch::DsEf,
            4 => Pitch::E,
            5 => Pitch::F,
            6 => Pitch::FsGf,
            7 => Pitch::G,
            8 => Pitch::GsAf,
            9 => Pitch::A,
            10 => Pitch::AsBf,
            11 => Pitch::B,
            _ => Pitch::None,
        };

        let octave = match octave_num {
            -1 => Octave::O_1,
            0 => Octave::O0,
            1 => Octave::O1,
            2 => Octave::O2,
            3 => Octave::O3,
            4 => Octave::O4,
            5 => Octave::O5,
            6 => Octave::O6,
            7 => Octave::O7,
            8 => Octave::O8,
            9 => Octave::O9,
            _ => Octave::O4,
        };

        (pitch, octave)
    }

    /// Transfers the active monophonic note from MPM to polyphonic CRNN tracking
    /// when switching from monophonic to CRNN in a hybrid session.
    pub fn transfer_mono_to_poly(&mut self) {
        if let Some(mono) = self.active_mono_note.take() {
            if let Some(idx) = Self::note_to_midi_index(mono.pitch, mono.octave) {
                self.active_notes[idx] = Some(mono);
            }
        }
    }

    /// Transfers the last active note in the CRNN that the MPM detects into the MPM active mono note,
    /// and finalizes all other polyphonic notes that were sounding.
    pub fn transfer_poly_to_mono(
        &mut self,
        target_note: Option<(Pitch, Octave)>,
        timestamp_ms: u128,
        profile: &InstrumentAcousticProfile,
    ) -> ArrayVec<EndNote, MAX_POLYPHONIC_NOTES> {
        let mut finalized = ArrayVec::new();

        let target_idx = target_note.and_then(|(p, o)| Self::note_to_midi_index(p, o));

        // 1. Move the matching detected note from polyphonic bins into active mono note
        if let Some(idx) = target_idx {
            if let Some(note) = self.active_notes[idx].take() {
                self.active_mono_note = Some(note);
            }
        }

        // 2. Finalize all remaining active notes in the polyphonic tracker
        for opt_note in self.active_notes.iter_mut() {
            if let Some(note) = opt_note.take() {
                let end_note = note.into_end_note_with_profile(timestamp_ms, false, profile);
                if end_note.note_duration >= self.min_note_duration_sec {
                    let _ = finalized.try_push(end_note);
                }
            }
        }

        finalized
    }

    /// Processes a single Native MPM frame for monophonic tracking.
    pub fn process_mpm_frame(
        &mut self,
        mpm_result: (f32, f32, f32), // (freq_hz, clarity, dbfs)
        hfc_onset_ts: Option<u128>,
        timestamp_ms: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
    ) -> ArrayVec<SegmentedNoteEvent, 2> {
        let mut events = ArrayVec::new();
        let (freq_hz, clarity, dbfs) = mpm_result;
        let detected = if clarity >= self.frame_threshold {
            get_note(freq_hz)
        } else {
            None
        };

        match (self.active_mono_note.as_mut(), detected) {
            // Case 1: No active note, but clear pitch detected -> START NOTE
            (None, Some((pitch, octave, tonality_offset))) => {
                let onset_ts = hfc_onset_ts.unwrap_or(timestamp_ms);
                let note = RecordNote::new_with_features(
                    pitch,
                    octave,
                    tonality_offset,
                    dbfs,
                    onset_ts,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    Some(clarity),
                );
                let start_event = note.to_start_note();
                self.active_mono_note = Some(note);
                let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
            }

            // Case 2: Active note exists, pitch matches -> UPDATE or RESTRIKE
            (Some(note), Some((pitch, octave, tonality_offset)))
                if note.pitch == pitch && note.octave == octave =>
            {
                if let Some(onset_ts) = hfc_onset_ts {
                    // HFC transient restrike on same pitch: finalize old note at onset_ts, start new note at onset_ts
                    let old_note = self.active_mono_note.take().unwrap();
                    let end_note = old_note.into_end_note_with_profile(onset_ts, false, profile);
                    if end_note.note_duration >= self.min_note_duration_sec {
                        let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                    }

                    let new_note = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        dbfs,
                        onset_ts,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        Some(clarity),
                    );
                    let start_event = new_note.to_start_note();
                    self.active_mono_note = Some(new_note);
                    let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                } else {
                    note.add_peak_dbfs(dbfs);
                    note.accumulate_frame(tonality_offset, spectral_centroid);
                }
            }

            // Case 3: Active note exists, pitch changed -> LEGATO TRANSITION
            (Some(_), Some((pitch, octave, tonality_offset))) => {
                let old_note = self.active_mono_note.take().unwrap();
                let end_note = old_note.into_end_note_with_profile(timestamp_ms, true, profile);
                if end_note.note_duration >= self.min_note_duration_sec {
                    let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                }

                let new_note = RecordNote::new_with_features(
                    pitch,
                    octave,
                    tonality_offset,
                    dbfs,
                    timestamp_ms,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    Some(clarity),
                );
                let start_event = new_note.to_start_note();
                self.active_mono_note = Some(new_note);
                let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
            }

            // Case 4: Active note exists, but clarity dropped -> RELEASE / END NOTE
            (Some(_), None) => {
                let old_note = self.active_mono_note.take().unwrap();
                let end_note = old_note.into_end_note_with_profile(timestamp_ms, false, profile);
                if end_note.note_duration >= self.min_note_duration_sec {
                    let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                }
            }

            // Case 5: No active note and no pitch -> Idle
            (None, None) => {}
        }

        events
    }

    /// Processes a single CRNN / Basic Pitch output frame for polyphonic tracking.
    pub fn process_crnn_frame(
        &mut self,
        output: &BasicPitchOutput,
        timestamp_ms: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
    ) -> ArrayVec<SegmentedNoteEvent, MAX_POLYPHONIC_NOTES> {
        let mut events = ArrayVec::new();

        for (idx, (&frame_prob, &onset_prob)) in
            output.frames.iter().zip(output.onsets.iter()).enumerate()
        {
            let is_onset = onset_prob >= self.onset_threshold;
            let is_frame_active = frame_prob >= self.frame_threshold;

            let (pitch, octave) = Self::midi_index_to_note(idx);
            let tonality_offset = 0i8;

            match (self.active_notes[idx].as_mut(), is_frame_active, is_onset) {
                // New Note Onset
                (None, true, true) | (None, true, false) => {
                    let note = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        output.energy_dbfs,
                        timestamp_ms,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        None,
                    );
                    let start_event = note.to_start_note();
                    self.active_notes[idx] = Some(note);
                    let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                }

                // Re-articulation / Restrike on already active note
                (Some(_), true, true) => {
                    let old_note = self.active_notes[idx].take().unwrap();
                    let end_note =
                        old_note.into_end_note_with_profile(timestamp_ms, false, profile);
                    if end_note.note_duration >= self.min_note_duration_sec {
                        let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                    }

                    let new_note = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        output.energy_dbfs,
                        timestamp_ms,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        None,
                    );
                    let start_event = new_note.to_start_note();
                    self.active_notes[idx] = Some(new_note);
                    let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                }

                // Note Sustain
                (Some(note), true, false) => {
                    note.add_peak_dbfs(output.energy_dbfs);
                    note.accumulate_frame(tonality_offset, spectral_centroid);
                }

                // Note Release
                (Some(_), false, _) => {
                    let old_note = self.active_notes[idx].take().unwrap();
                    let end_note =
                        old_note.into_end_note_with_profile(timestamp_ms, false, profile);
                    if end_note.note_duration >= self.min_note_duration_sec {
                        let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                    }
                }

                (None, false, _) => {}
            }
        }

        events
    }

    /// Finalizes all currently active notes (both monophonic and polyphonic) upon stream stop or silence.
    pub fn finalize_all(
        &mut self,
        timestamp_ms: u128,
        profile: &InstrumentAcousticProfile,
    ) -> ArrayVec<EndNote, MAX_POLYPHONIC_NOTES> {
        let mut finalized = ArrayVec::new();

        if let Some(mono) = self.active_mono_note.take() {
            let end_note = mono.into_end_note_with_profile(timestamp_ms, false, profile);
            if end_note.note_duration >= self.min_note_duration_sec {
                let _ = finalized.try_push(end_note);
            }
        }

        for opt_note in self.active_notes.iter_mut() {
            if let Some(note) = opt_note.take() {
                let end_note = note.into_end_note_with_profile(timestamp_ms, false, profile);
                if end_note.note_duration >= self.min_note_duration_sec {
                    let _ = finalized.try_push(end_note);
                }
            }
        }

        finalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_processing::instruments::instrument::Instrument;

    #[test]
    fn test_midi_pitch_conversion() {
        assert_eq!(
            StreamingNoteSegmenter::note_to_midi_index(Pitch::A, Octave::O4),
            Some(48)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(48),
            (Pitch::A, Octave::O4)
        );

        assert_eq!(
            StreamingNoteSegmenter::note_to_midi_index(Pitch::C, Octave::O4),
            Some(39)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(39),
            (Pitch::C, Octave::O4)
        );
    }

    #[test]
    fn test_segmenter_start_and_end() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4);
        let profile = Instrument::Piano.acoustic_profile();

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
        assert!(matches!(event1[0], SegmentedNoteEvent::Start(_)));

        let event2 =
            segmenter.process_mpm_frame((0.0, 0.1, -50.0), None, 1200, 4.0, -60.0, 500.0, &profile);
        assert_eq!(event2.len(), 1);
        assert!(matches!(event2[0], SegmentedNoteEvent::End(_)));
    }

    #[test]
    fn test_segmenter_restrike() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4);
        let profile = Instrument::Piano.acoustic_profile();

        let _ = segmenter.process_mpm_frame(
            (440.0, 0.95, -12.0),
            None,
            1000,
            12.0,
            -60.0,
            1500.0,
            &profile,
        );

        let event2 = segmenter.process_mpm_frame(
            (440.0, 0.95, -10.0),
            Some(1065), // Restrike with confirmed peak timestamp at 1065ms
            1100,
            14.0,
            -60.0,
            2000.0,
            &profile,
        );
        assert_eq!(event2.len(), 2);
        match &event2[0] {
            SegmentedNoteEvent::End(e) => {
                assert_eq!(e.pitch, Pitch::A);
                assert_eq!(e.note_striked, 1000);
                assert!((e.note_duration - 0.065).abs() < 0.001);
            }
            _ => panic!("Expected EndNote on restrike"),
        }
        match &event2[1] {
            SegmentedNoteEvent::Start(s) => {
                assert_eq!(s.pitch, Pitch::A);
                assert_eq!(s.note_striked, 1065);
            }
            _ => panic!("Expected StartNote on restrike"),
        }
    }
}
