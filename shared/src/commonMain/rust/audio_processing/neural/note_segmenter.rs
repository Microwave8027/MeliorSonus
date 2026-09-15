use crate::audio_processing::instruments::instrument::InstrumentAcousticProfile;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::bytedance_crnn::ByteDanceCrnnOutput;
use crate::audio_processing::neural::crnn::{
    BasicPitchOutput, MIDI_OFFSET, NUM_MIDI_NOTES, NUM_PITCH_BINS,
};
use crate::audio_processing::processing::functions::spectral::psychoacoustic_loudness::PitchLoudness;
use crate::audio_processing::{PsychoacousticLoudnessMeter, classify_articulation};
use crate::constants::FRAME_SIZE;
use arrayvec::ArrayVec;

pub const MAX_POLYPHONIC_NOTES: usize = NUM_MIDI_NOTES * 2;
pub const MONO_ENTRY_CLARITY: f32 = 0.80;
pub const MONO_EXIT_CLARITY: f32 = 0.55;

#[derive(Clone, Debug, PartialEq)]
pub enum SegmentedNoteEvent {
    Start(StartNote),
    End(EndNote),
}

fn dynamic_from_velocity(vel: u8) -> DynamicLevel {
    match vel {
        0..=31 => DynamicLevel::Pianississimo,
        32..=47 => DynamicLevel::Pianissimo,
        48..=63 => DynamicLevel::Piano,
        64..=79 => DynamicLevel::MezzoPiano,
        80..=95 => DynamicLevel::MezzoForte,
        96..=111 => DynamicLevel::Forte,
        112..=120 => DynamicLevel::Fortissimo,
        _ => DynamicLevel::Fortississimo,
    }
}

pub struct StreamingNoteSegmenter {
    active_notes: [Option<RecordNote>; NUM_MIDI_NOTES],
    active_mono_note: Option<RecordNote>,
    psychoacoustic_loudness: PsychoacousticLoudnessMeter,
    onset_threshold: f32,
    frame_threshold: f32,
    min_note_duration_sec: f32,

    // Monophonic MPM Anti-Fragmentation & Elevated Thresholds
    /// Entry / lock clarity required to confirm a new pitch (default: 0.80)
    mono_entry_clarity: f32,
    /// Exit / sustain floor below which hangover countdown begins (default: 0.55)
    mono_exit_clarity: f32,
    /// Frames remaining before an active note is officially terminated after clarity drops
    mono_hangover_frames: u8,
    /// Timestamp when clarity first dropped below threshold
    mono_hangover_start_ts: u128,
    /// Minimum hangover frames to bridge momentary dips (default: 3 frames ~35ms)
    mono_hangover_default: u8,
    /// Pitch hysteresis deadband in semitones (0.70 = +/- 70 cents)
    pitch_hysteresis_semitones: f32,
    /// Candidate pitch for legato transition requiring multi-frame confirmation
    pending_legato_candidate: Option<(Pitch, Octave, i8, u8)>, // (pitch, octave, offset, consecutive_frames)

    // ByteDance CRNN Dedicated State Tracking (0 heap allocation)
    bytedance_velocity: [u8; NUM_MIDI_NOTES],
    bytedance_restrike_lockout: [u8; NUM_MIDI_NOTES],
    bytedance_pedal_alive: [bool; NUM_MIDI_NOTES],
    bytedance_prev_onset: [f32; NUM_MIDI_NOTES],
    bytedance_pedal_active: bool,
    bytedance_pedal_hangover: u8,
}

impl StreamingNoteSegmenter {
    pub fn new(onset_threshold: f32, frame_threshold: f32, sampling_rate: u32) -> Self {
        let mono_entry_clarity = MONO_ENTRY_CLARITY.max(frame_threshold);
        let mono_exit_clarity = MONO_EXIT_CLARITY.min(frame_threshold);

        let psychoacoustic_loudness = PsychoacousticLoudnessMeter::<FRAME_SIZE>::new(sampling_rate);
        Self {
            active_notes: core::array::from_fn(|_| None),
            active_mono_note: None,
            onset_threshold,
            frame_threshold,
            min_note_duration_sec: 0.020,
            mono_entry_clarity,
            mono_exit_clarity,
            mono_hangover_frames: 0,
            mono_hangover_start_ts: 0,
            mono_hangover_default: 3,
            pitch_hysteresis_semitones: 0.70,
            pending_legato_candidate: None,
            psychoacoustic_loudness,
            bytedance_velocity: [0; NUM_MIDI_NOTES],
            bytedance_restrike_lockout: [0; NUM_MIDI_NOTES],
            bytedance_pedal_alive: [false; NUM_MIDI_NOTES],
            bytedance_prev_onset: [0.0; NUM_MIDI_NOTES],
            bytedance_pedal_active: false,
            bytedance_pedal_hangover: 0,
        }
    }

    pub fn set_mono_thresholds(&mut self, entry_clarity: f32, exit_clarity: f32) {
        self.mono_entry_clarity = entry_clarity;
        self.mono_exit_clarity = exit_clarity;
    }

    pub fn set_hangover_frames(&mut self, default_frames: u8) {
        self.mono_hangover_default = default_frames;
    }

    pub fn set_pitch_hysteresis(&mut self, semitones: f32) {
        self.pitch_hysteresis_semitones = semitones;
    }

    pub fn frame_threshold(&self) -> f32 {
        self.frame_threshold
    }

    pub fn mono_entry_clarity(&self) -> f32 {
        self.mono_entry_clarity
    }

    pub fn mono_exit_clarity(&self) -> f32 {
        self.mono_exit_clarity
    }

    pub fn active_mono_note(&self) -> Option<&RecordNote> {
        self.active_mono_note.as_ref()
    }

    pub fn active_notes(&self) -> &[Option<RecordNote>; NUM_MIDI_NOTES] {
        &self.active_notes
    }

    pub fn bytedance_pedal_alive(&self) -> &[bool; NUM_MIDI_NOTES] {
        &self.bytedance_pedal_alive
    }

    /// Converts a frequency in Hz into a global MIDI note number (0..127).
    pub fn freq_to_midi_index(freq: f32) -> Option<usize> {
        if freq <= 0.0 || freq.is_nan() || freq.is_infinite() {
            return None;
        }
        let n = 69.0 + 12.0 * (freq / 440.0).log2();
        if !(0.0..=127.0).contains(&n) {
            return None;
        }
        Some(n.round() as usize)
    }

    /// Converts a (Pitch, Octave) into a global MIDI note number (0..127).
    pub fn note_to_midi_index(pitch: Pitch, octave: Octave) -> Option<usize> {
        pitch_octave_to_midi(pitch, octave).map(|m| m as usize)
    }

    /// Converts a global MIDI note number (0..127) back into (Pitch, Octave).
    pub fn midi_index_to_note(idx: usize) -> (Pitch, Octave) {
        (Pitch::get_pitch(idx as u8), Octave::midi_to_note(idx as u8))
    }

    /// Transfers the active monophonic note from MPM to polyphonic CRNN tracking
    /// when switching from monophonic to CRNN in a hybrid session.
    pub fn transfer_mono_to_poly(&mut self) {
        self.mono_hangover_frames = 0;
        self.pending_legato_candidate = None;
        self.bytedance_pedal_alive = [false; NUM_MIDI_NOTES];
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
        self.mono_hangover_frames = 0;
        self.pending_legato_candidate = None;
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

        self.bytedance_pedal_alive = [false; NUM_MIDI_NOTES];
        self.bytedance_pedal_active = false;
        self.bytedance_velocity = [0; NUM_MIDI_NOTES];

        finalized
    }

    /// Processes a single Native MPM frame for monophonic tracking with pitch hysteresis,
    /// release hangover countdown, and debounced restrikes.
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

        // 1. Continuous fractional MIDI pitch calculation
        let continuous_midi =
            if freq_hz > 20.0 && freq_hz < 4500.0 && !freq_hz.is_nan() && !freq_hz.is_infinite() {
                Some(69.0 + 12.0 * (freq_hz / 440.0).log2())
            } else {
                None
            };

        // 2. Discrete pitch candidate (only when clarity >= mono_entry_clarity)
        let detected_pitch_candidate = if clarity >= self.mono_entry_clarity {
            get_note(freq_hz)
        } else {
            None
        };

        if let Some(active_note) = self.active_mono_note.as_mut() {
            let active_nominal_midi =
                Self::note_to_midi_index(active_note.pitch, active_note.octave)
                    .map(|midi| midi as f32);

            let is_voiced =
                clarity >= self.mono_exit_clarity && continuous_midi.is_some() && dbfs >= -65.0;

            if is_voiced {
                let midi_val = continuous_midi.unwrap();
                let semitone_diff = if let Some(active_midi) = active_nominal_midi {
                    (midi_val - active_midi).abs()
                } else {
                    999.0
                };

                // Reset any hangover countdown since note is voiced & healthy
                self.mono_hangover_frames = 0;

                if semitone_diff <= self.pitch_hysteresis_semitones {
                    // SAME NOTE: Pitch is within hysteresis deadband (+/- 70 cents)
                    self.pending_legato_candidate = None;

                    if let Some(onset_ts) = hfc_onset_ts {
                        // A true restrike requires that the new onset timestamp is >= 80ms after the active note was struck
                        if onset_ts.saturating_sub(active_note.note_striked) >= 80 {
                            let old_note = self.active_mono_note.take().unwrap();
                            let end_note =
                                old_note.into_end_note_with_profile(onset_ts, false, profile);
                            if end_note.note_duration >= self.min_note_duration_sec {
                                let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                            }

                            let (pitch, octave, tonality_offset) = detected_pitch_candidate
                                .unwrap_or_else(|| {
                                    let rounded_midi = midi_val.round() as u8;
                                    (
                                        Pitch::get_pitch(rounded_midi),
                                        Octave::midi_to_note(rounded_midi),
                                        ((midi_val - rounded_midi as f32) * 100.0)
                                            .clamp(-128.0, 127.0)
                                            .round() as i8,
                                    )
                                });

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
                                None,
                                None,
                            );
                            let start_event = new_note.to_start_note();
                            self.active_mono_note = Some(new_note);
                            let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                        } else {
                            // Onset is within 80ms of note start (the initial strike, rebound, or body resonance).
                            // Retroactively refine note_striked to the precise onset_ts if earlier, without splitting.
                            if onset_ts < active_note.note_striked {
                                active_note.note_striked = onset_ts;
                            }
                            active_note.add_peak_dbfs(dbfs);
                            let tonality_offset = ((midi_val - midi_val.round()) * 100.0)
                                .clamp(-128.0, 127.0)
                                .round() as i8;
                            active_note.accumulate_frame(tonality_offset, spectral_centroid, None);
                            active_note.last_timestamp = timestamp_ms;
                        }
                    } else {
                        // Normal steady-state sustain
                        active_note.add_peak_dbfs(dbfs);
                        let tonality_offset = ((midi_val - midi_val.round()) * 100.0)
                            .clamp(-128.0, 127.0)
                            .round() as i8;
                        active_note.accumulate_frame(tonality_offset, spectral_centroid, None);
                        active_note.last_timestamp = timestamp_ms;
                    }
                } else if let Some((new_pitch, new_octave, new_offset)) = detected_pitch_candidate {
                    // PITCH JUMP BEYOND HYSTERESIS:
                    let note_age_ms = timestamp_ms.saturating_sub(active_note.note_striked);
                    if note_age_ms < 50 {
                        // Early attack transient pitch clarification: adopt true stabilized fundamental without splitting into two notes!
                        active_note.pitch = new_pitch;
                        active_note.octave = new_octave;
                        active_note.tonality_offset = new_offset;
                        active_note.add_peak_dbfs(dbfs);
                        active_note.accumulate_frame(new_offset, spectral_centroid, None);
                        active_note.last_timestamp = timestamp_ms;
                        self.pending_legato_candidate = None;
                    } else {
                        // Legato transition on an established sustained note
                        let is_matching_candidate = match self.pending_legato_candidate {
                            Some((p, o, _, _)) => p == new_pitch && o == new_octave,
                            None => false,
                        };

                        if is_matching_candidate {
                            let count = self
                                .pending_legato_candidate
                                .map(|(_, _, _, c)| c)
                                .unwrap_or(0)
                                + 1;
                            if count >= 2 {
                                // Confirmed legato transition!
                                self.pending_legato_candidate = None;
                                let old_note = self.active_mono_note.take().unwrap();
                                let end_note = old_note.into_end_note_with_profile(
                                    timestamp_ms,
                                    true,
                                    profile,
                                );
                                if end_note.note_duration >= self.min_note_duration_sec {
                                    let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                                }

                                let new_note = RecordNote::new_with_features(
                                    new_pitch,
                                    new_octave,
                                    new_offset,
                                    dbfs,
                                    timestamp_ms,
                                    crest_factor,
                                    sub_thump_dbfs,
                                    spectral_centroid,
                                    Some(clarity),
                                    None,
                                    None,
                                );
                                let start_event = new_note.to_start_note();
                                self.active_mono_note = Some(new_note);
                                let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                            } else {
                                self.pending_legato_candidate =
                                    Some((new_pitch, new_octave, new_offset, count));
                                active_note.last_timestamp = timestamp_ms;
                            }
                        } else {
                            // First frame of new pitch candidate: hold off and accumulate
                            self.pending_legato_candidate =
                                Some((new_pitch, new_octave, new_offset, 1));
                            active_note.last_timestamp = timestamp_ms;
                        }
                    }
                } else {
                    // Frame is voiced but below entry clarity for a new pitch: sustain active note
                    active_note.last_timestamp = timestamp_ms;
                }
            } else {
                // UNVOICED / LOW CLARITY / SILENCE -> Release Hangover countdown
                self.pending_legato_candidate = None;

                let last_ts = active_note.last_timestamp;
                let is_large_gap = timestamp_ms.saturating_sub(last_ts) > 60;

                if self.mono_hangover_frames == 0 && !is_large_gap {
                    // First unvoiced frame: start hangover countdown
                    self.mono_hangover_frames = self.mono_hangover_default;
                    self.mono_hangover_start_ts = timestamp_ms;
                } else if self.mono_hangover_frames > 0 && !is_large_gap {
                    self.mono_hangover_frames -= 1;
                    if self.mono_hangover_frames == 0 {
                        // Hangover expired
                        let old_note = self.active_mono_note.take().unwrap();
                        let end_note = old_note.into_end_note_with_profile(
                            self.mono_hangover_start_ts,
                            false,
                            profile,
                        );
                        if end_note.note_duration >= self.min_note_duration_sec {
                            let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                        }
                    }
                } else {
                    // Large timestamp gap or already expired -> finalize immediately
                    self.mono_hangover_frames = 0;
                    let old_note = self.active_mono_note.take().unwrap();
                    let end_note =
                        old_note.into_end_note_with_profile(timestamp_ms, false, profile);
                    if end_note.note_duration >= self.min_note_duration_sec {
                        let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                    }
                }
            }
        } else {
            // NO ACTIVE NOTE (IDLE)
            self.mono_hangover_frames = 0;
            self.pending_legato_candidate = None;

            if let Some((pitch, octave, tonality_offset)) = detected_pitch_candidate {
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
                    None,
                    None,
                );
                let start_event = note.to_start_note();
                self.active_mono_note = Some(note);
                let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
            }
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
        frame: &[f32],
    ) -> ArrayVec<SegmentedNoteEvent, MAX_POLYPHONIC_NOTES> {
        let mut events = ArrayVec::new();

        let mut valid_frequencies = ArrayVec::<f32, NUM_PITCH_BINS>::new();
        for (idx, &frame_prob) in output.frames.iter().enumerate() {
            let midi_idx = idx + MIDI_OFFSET;
            let is_onset = output
                .onsets
                .get(idx)
                .map_or(false, |&p| p >= self.onset_threshold);
            let is_active = self
                .active_notes
                .get(midi_idx)
                .and_then(|n| n.as_ref())
                .is_some();
            if frame_prob >= self.frame_threshold || is_onset || is_active {
                let _ = valid_frequencies.try_push(midi_to_freq(midi_idx as u8));
            }
        }
        let loudness_values = self
            .psychoacoustic_loudness
            .calculate_frequency_loudness(frame, &valid_frequencies[..]);

        for (idx, &onset_prob) in output.onsets.iter().enumerate() {
            let midi_idx = idx + MIDI_OFFSET;
            let is_onset = onset_prob >= self.onset_threshold;
            let frame_prob = output.frames.get(idx).copied().unwrap_or(0.0);
            let is_frame_active = frame_prob >= self.frame_threshold;

            let loudness_opt = loudness_values.get(midi_idx).and_then(|&opt| opt);
            let sones = loudness_opt.map(|l| l.sones);
            let phons = loudness_opt.map(|l| l.phons);

            let (pitch, octave) = Self::midi_index_to_note(midi_idx);
            let tonality_offset = 0i8;

            match (
                self.active_notes[midi_idx].as_mut(),
                is_frame_active,
                is_onset,
            ) {
                // New Note Onset
                (None, true, _) => {
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
                        sones,
                        phons,
                    );
                    let start_event = note.to_start_note();
                    self.active_notes[midi_idx] = Some(note);
                    let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                }

                // Re-articulation / Restrike on already active note
                (Some(_), true, true) => {
                    let old_note = self.active_notes[midi_idx].take().unwrap();
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
                        sones,
                        phons,
                    );
                    let start_event = new_note.to_start_note();
                    self.active_notes[midi_idx] = Some(new_note);
                    let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                }

                // Note Sustain
                (Some(note), true, false) => {
                    note.add_peak_dbfs(output.energy_dbfs);
                    note.accumulate_frame(tonality_offset, spectral_centroid, sones);
                }

                // Note Release
                (Some(_), false, _) => {
                    let old_note = self.active_notes[midi_idx].take().unwrap();
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
    /// bytedance helpers
    ///
    /// Updates the global sustain pedal state using dual Schmitt-trigger hysteresis
    /// and a 2-frame (40ms) release hangover filter.
    /// Returns `true` if the pedal was just released on this frame.
    fn update_bytedance_pedal_state(&mut self, output: &ByteDanceCrnnOutput) -> bool {
        let pedal_on = output.pedal_frame >= 0.50 || output.pedal_onset >= 0.40;
        let pedal_off = output.pedal_frame < 0.35 || output.pedal_offset >= 0.35;

        let prev_pedal = self.bytedance_pedal_active;
        if pedal_on {
            self.bytedance_pedal_active = true;
            self.bytedance_pedal_hangover = 0;
        } else if self.bytedance_pedal_active && pedal_off {
            self.bytedance_pedal_active = false;
            self.bytedance_pedal_hangover = 0;
        }
        let pedal_just_released = prev_pedal && !self.bytedance_pedal_active;
        if !self.bytedance_pedal_active {
            self.bytedance_pedal_alive = [false; NUM_MIDI_NOTES];
        }
        pedal_just_released
    }

    /// Dispatches optional Zwicker Bark sones and phons computation for active frequencies
    /// when a non-empty audio frame slice is passed.
    #[allow(dead_code)]
    fn calculate_bytedance_loudness(
        &mut self,
        output: &ByteDanceCrnnOutput,
        frame: &[f32],
    ) -> [Option<PitchLoudness>; NUM_MIDI_NOTES] {
        if frame.is_empty() {
            return [None; NUM_MIDI_NOTES];
        }

        let mut valid_frequencies = ArrayVec::<f32, NUM_PITCH_BINS>::new();
        for (idx, &frame_prob) in output.frames.iter().enumerate() {
            let midi_idx = idx + MIDI_OFFSET;
            let is_onset = output
                .onsets
                .get(idx)
                .map_or(false, |&p| p >= self.onset_threshold);
            let is_active = self
                .active_notes
                .get(midi_idx)
                .and_then(|n| n.as_ref())
                .is_some();
            if frame_prob >= self.frame_threshold || is_onset || is_active {
                let _ = valid_frequencies.try_push(midi_to_freq(midi_idx as u8));
            }
        }
        self.psychoacoustic_loudness
            .calculate_frequency_loudness(frame, &valid_frequencies[..])
    }

    /// Initializes active note tracking for a key, overrides neural velocity & dynamic,
    /// sets initial rise duration, stores the note in `active_notes`, and returns `StartNote`.
    fn start_bytedance_note(
        &mut self,
        midi_idx: usize,
        pitch: Pitch,
        octave: Octave,
        velocity: u8,
        timestamp_ms: u128,
        energy_dbfs: f32,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        sones: Option<f32>,
        phons: Option<f32>,
    ) -> StartNote {
        const RESTRIKE_LOCKOUT_FRAMES: u8 = 4; // 80ms lockout @ 20ms hop

        self.bytedance_velocity[midi_idx] = velocity;
        self.bytedance_restrike_lockout[midi_idx] = RESTRIKE_LOCKOUT_FRAMES;
        self.bytedance_pedal_alive[midi_idx] = false;

        let mut note = RecordNote::new_with_features(
            pitch,
            octave,
            0, // tonality_offset
            energy_dbfs,
            timestamp_ms,
            crest_factor,
            sub_thump_dbfs,
            spectral_centroid,
            None,
            sones,
            phons,
        );
        note.rise_duration = Some(0.020);

        let mut start_event = note.to_start_note();
        start_event.velocity = Some(velocity);
        start_event.dynamic = dynamic_from_velocity(velocity);

        self.active_notes[midi_idx] = Some(note);
        start_event
    }

    /// Extracts an active note, applies neural velocity, dynamic, recomputed articulation,
    /// and optional damping override, then returns `Some(EndNote)` if above minimum duration.
    fn finalize_bytedance_note(
        &mut self,
        midi_idx: usize,
        timestamp_ms: u128,
        damping_override: Option<DampingProfile>,
        profile: &InstrumentAcousticProfile,
    ) -> Option<EndNote> {
        let old_note = self.active_notes[midi_idx].take()?;
        let vel = self.bytedance_velocity[midi_idx];
        self.bytedance_velocity[midi_idx] = 0;

        let initial_crest_factor = old_note.initial_crest_factor;
        let effective_rise = old_note.rise_duration.unwrap_or(0.020).max(0.005);
        let attack_slope = (old_note.peak_dbfs - old_note.onset_dbfs).abs() / effective_rise;

        let mut end_note = old_note.into_end_note_with_profile(timestamp_ms, false, profile);
        if vel > 0 {
            end_note.velocity = Some(vel);
            end_note.dynamic = dynamic_from_velocity(vel);
            end_note.articulation = classify_articulation(
                end_note.note_duration,
                false,
                attack_slope,
                effective_rise,
                initial_crest_factor,
                vel,
                profile,
            );
        }
        if let Some(d) = damping_override {
            end_note.damping = d;
        }

        if end_note.note_duration >= self.min_note_duration_sec {
            Some(end_note)
        } else {
            None
        }
    }

    /// Accumulates sustain frame energy, spectral centroid, and sones on an active note.
    fn accumulate_bytedance_sustain(
        note: &mut RecordNote,
        timestamp_ms: u128,
        energy_dbfs: f32,
        spectral_centroid: f32,
        sones: Option<f32>,
    ) {
        note.last_timestamp = timestamp_ms;
        note.add_peak_dbfs(energy_dbfs);
        note.accumulate_frame(0, spectral_centroid, sones);
        if note.rise_duration.is_none() {
            note.rise_duration = Some(0.020);
        }
    }

    /// Processes a single ByteDance CRNN output frame with 7 heads (onsets, offsets, frames, velocity, pedals).
    pub fn process_bytedance_crnn_frame(
        &mut self,
        output: &ByteDanceCrnnOutput,
        timestamp_ms: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
        _frame: &[f32], // Used only when calcululating loudness with phons and sones, uneeded here.
    ) -> ArrayVec<SegmentedNoteEvent, MAX_POLYPHONIC_NOTES> {
        let mut events = ArrayVec::new();

        const OFFSET_THRESHOLD: f32 = 0.35;
        const LEGATO_FRAME_THRESHOLD: f32 = 0.65;

        // Update Global Pedal State (Dual Schmitt-Trigger + 40ms Hangover)
        let _pedal_just_released = self.update_bytedance_pedal_state(output);

        // Optional Psychoacoustic Loudness (Zwicker Bark Sones/Phons)
        // let loudness_values = self.calculate_bytedance_loudness(output, _frame);

        // Per-Key State Machine (88 Keys: MIDI 21 A0 to 108 C8)
        for (idx, &onset_prob) in output.onsets.iter().enumerate() {
            let midi_idx = idx + MIDI_OFFSET;
            if self.bytedance_restrike_lockout[midi_idx] > 0 {
                self.bytedance_restrike_lockout[midi_idx] -= 1;
            }

            let frame_prob = output.frames.get(idx).copied().unwrap_or(0.0);
            let offset_prob = output.offsets.get(idx).copied().unwrap_or(0.0);
            let neural_velocity = output.velocity.get(idx).copied().unwrap_or(64.0);

            // Rising-edge peak detection
            let prev_onset = self.bytedance_prev_onset[midi_idx];
            let is_onset_spike = onset_prob >= self.onset_threshold && onset_prob >= prev_onset;
            self.bytedance_prev_onset[midi_idx] = onset_prob;

            let is_frame_active = frame_prob >= self.frame_threshold;
            let is_offset_spike = offset_prob >= OFFSET_THRESHOLD;

            let safe_vel = if neural_velocity.is_nan() {
                64.0
            } else {
                neural_velocity
            };
            let vel_midi = safe_vel.clamp(1.0, 127.0).round() as u8;

            // let loudness_opt = loudness_values.get(midi_idx).and_then(|&opt| opt);
            // let sones = loudness_opt.map(|l| l.sones);
            // let phons = loudness_opt.map(|l| l.phons);

            let (pitch, octave) = Self::midi_index_to_note(midi_idx);

            match self.active_notes[midi_idx].as_mut() {
                // CASE 1: IDLE KEY -> NOTE START
                None => {
                    let is_pedal_alive = self.bytedance_pedal_alive[midi_idx];
                    let is_debounced_onset =
                        is_onset_spike && self.bytedance_restrike_lockout[midi_idx] == 0;
                    let should_start = if is_pedal_alive {
                        // When marked in pedal_alive, ignore all activations unless a new physical onset is detected
                        is_debounced_onset
                    } else {
                        is_debounced_onset
                            || (frame_prob >= LEGATO_FRAME_THRESHOLD
                                && !is_offset_spike
                                && output.energy_dbfs > -50.0)
                    };

                    if should_start {
                        self.bytedance_pedal_alive[midi_idx] = false;
                        let start_event = self.start_bytedance_note(
                            midi_idx,
                            pitch,
                            octave,
                            vel_midi,
                            timestamp_ms,
                            output.energy_dbfs,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            None, // sones
                            None, // phons
                        );
                        let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                    }
                }

                // CASE 2: ACTIVE KEY -> SUSTAIN / RESTRIKE / RELEASE
                Some(note) => {
                    Self::accumulate_bytedance_sustain(
                        note,
                        timestamp_ms,
                        output.energy_dbfs,
                        spectral_centroid,
                        None, // sones
                    );

                    // CONFIRMED RESTRIKE (debounced >= 80ms)
                    if is_onset_spike && self.bytedance_restrike_lockout[midi_idx] == 0 {
                        if let Some(end_note) =
                            self.finalize_bytedance_note(midi_idx, timestamp_ms, None, profile)
                        {
                            let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                        }
                        self.bytedance_pedal_alive[midi_idx] = false;
                        let start_event = self.start_bytedance_note(
                            midi_idx,
                            pitch,
                            octave,
                            vel_midi,
                            timestamp_ms,
                            output.energy_dbfs,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            None, // sones
                            None, // phons
                        );
                        let _ = events.try_push(SegmentedNoteEvent::Start(start_event));
                    }
                    // KEY RELEASE: Musician lets go of key (offset spike or frame dropped)
                    else if is_offset_spike || !is_frame_active {
                        let damping = if self.bytedance_pedal_active {
                            self.bytedance_pedal_alive[midi_idx] = true;
                            DampingProfile::PedalSustained
                        } else {
                            self.bytedance_pedal_alive[midi_idx] = false;
                            DampingProfile::DryDamped
                        };

                        if let Some(end_note) = self.finalize_bytedance_note(
                            midi_idx,
                            timestamp_ms,
                            Some(damping),
                            profile,
                        ) {
                            let _ = events.try_push(SegmentedNoteEvent::End(end_note));
                        }
                    }
                }
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

        for midi_idx in 0..NUM_MIDI_NOTES {
            if let Some(end_note) =
                self.finalize_bytedance_note(midi_idx, timestamp_ms, None, profile)
            {
                let _ = finalized.try_push(end_note);
            }
        }
        self.bytedance_pedal_alive = [false; NUM_MIDI_NOTES];
        self.bytedance_pedal_active = false;
        self.bytedance_velocity = [0; NUM_MIDI_NOTES];

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
            Some(69)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(69),
            (Pitch::A, Octave::O4)
        );

        assert_eq!(
            StreamingNoteSegmenter::note_to_midi_index(Pitch::C, Octave::O4),
            Some(60)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(60),
            (Pitch::C, Octave::O4)
        );

        // Boundary tests across 0..127 global MIDI scale
        assert_eq!(
            StreamingNoteSegmenter::note_to_midi_index(Pitch::C, Octave::O_1),
            Some(0)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(0),
            (Pitch::C, Octave::O_1)
        );

        assert_eq!(
            StreamingNoteSegmenter::note_to_midi_index(Pitch::G, Octave::O9),
            Some(127)
        );
        assert_eq!(
            StreamingNoteSegmenter::midi_index_to_note(127),
            (Pitch::G, Octave::O9)
        );

        assert_eq!(StreamingNoteSegmenter::freq_to_midi_index(440.0), Some(69));
        assert_eq!(StreamingNoteSegmenter::freq_to_midi_index(261.63), Some(60));
    }

    #[test]
    fn test_segmenter_start_and_end() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
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
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
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
            Some(1090), // Restrike with confirmed peak timestamp at 1090ms (90ms >= 80ms restrike debounce)
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
                assert!((e.note_duration - 0.090).abs() < 0.001);
            }
            _ => panic!("Expected EndNote on restrike"),
        }
        match &event2[1] {
            SegmentedNoteEvent::Start(s) => {
                assert_eq!(s.pitch, Pitch::A);
                assert_eq!(s.note_striked, 1090);
            }
            _ => panic!("Expected StartNote on restrike"),
        }
    }

    #[test]
    fn test_mpm_loud_strike_early_attack_pitch_clarification() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // 1. Loud strike attack at ts=1000: Initial overtone detected (A5 at 880 Hz)
        let event1 = segmenter.process_mpm_frame(
            (880.0, 0.85, -6.0),
            Some(1000),
            1000,
            18.0,
            -50.0,
            3000.0,
            &profile,
        );
        assert_eq!(event1.len(), 1);
        assert!(matches!(event1[0], SegmentedNoteEvent::Start(_)));

        // 2. Frame at ts=1015 (15ms after start < 50ms): Pitch clarifies to fundamental A4 (440 Hz)
        let event2 = segmenter.process_mpm_frame(
            (440.0, 0.92, -8.0),
            None,
            1015,
            12.0,
            -60.0,
            1800.0,
            &profile,
        );
        // Early attack pitch adoption must update the note without emitting a legato split (0 events)
        assert!(
            event2.is_empty(),
            "Early attack overtone clarification must not split note"
        );
        assert_eq!(segmenter.active_mono_note().unwrap().pitch, Pitch::A);
        assert_eq!(segmenter.active_mono_note().unwrap().octave, Octave::O4);
        assert_eq!(segmenter.active_mono_note().unwrap().note_striked, 1000);
    }

    #[test]
    fn test_mpm_vibrato_hysteresis_does_not_fragment() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // 1. Initial lock at 440.0 Hz (A4, 0 cents)
        let start = segmenter.process_mpm_frame(
            (440.0, 0.92, -12.0),
            None,
            1000,
            12.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert_eq!(start.len(), 1);
        assert!(matches!(start[0], SegmentedNoteEvent::Start(_)));

        // 2. Vibrato sweep upward to 450 Hz (+39 cents, close to semitone boundary)
        let vib1 = segmenter.process_mpm_frame(
            (450.0, 0.88, -13.0),
            None,
            1015,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert!(vib1.is_empty(), "Vibrato sweep must not emit note events");

        // 3. Vibrato sweep downward to 430 Hz (-40 cents)
        let vib2 = segmenter.process_mpm_frame(
            (430.0, 0.89, -13.0),
            None,
            1030,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert!(
            vib2.is_empty(),
            "Vibrato sweep downward must not emit note events"
        );

        // Assert note is still single continuous active note
        assert!(segmenter.active_mono_note().is_some());
        let active = segmenter.active_mono_note().unwrap();
        assert_eq!(active.pitch, Pitch::A);
        assert_eq!(active.octave, Octave::O4);
    }

    #[test]
    fn test_mpm_clarity_hangover_bridges_gap() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // 1. Start note at 440 Hz
        let _ = segmenter.process_mpm_frame(
            (440.0, 0.95, -12.0),
            None,
            1000,
            12.0,
            -60.0,
            1500.0,
            &profile,
        );

        // 2. Single-frame clarity collapse (e.g. 0.20 clarity at ts=1012ms)
        let dip = segmenter.process_mpm_frame(
            (440.0, 0.20, -18.0),
            None,
            1012,
            6.0,
            -60.0,
            1200.0,
            &profile,
        );
        assert!(
            dip.is_empty(),
            "Single-frame clarity dip must be absorbed by hangover"
        );
        assert!(segmenter.active_mono_note().is_some());

        // 3. Clarity recovers on next frame (ts=1024ms)
        let rec = segmenter.process_mpm_frame(
            (440.0, 0.92, -14.0),
            None,
            1024,
            10.0,
            -60.0,
            1400.0,
            &profile,
        );
        assert!(
            rec.is_empty(),
            "Recovery must continue seamless sustain without re-triggering"
        );
        assert!(segmenter.active_mono_note().is_some());
    }

    #[test]
    fn test_mpm_legato_requires_confirmation() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // 1. Start on A4 (440 Hz)
        let _ = segmenter.process_mpm_frame(
            (440.0, 0.95, -12.0),
            None,
            1000,
            12.0,
            -60.0,
            1500.0,
            &profile,
        );

        // 2. Glitch frame: Jump to C5 (523.25 Hz) for a single frame at ts=1012
        let glitch = segmenter.process_mpm_frame(
            (523.25, 0.90, -14.0),
            None,
            1012,
            10.0,
            -60.0,
            1600.0,
            &profile,
        );
        assert!(
            glitch.is_empty(),
            "Single glitch frame must not immediately trigger legato"
        );

        // 3. Glitch disappears, returns to A4 on ts=1024
        let return_frame = segmenter.process_mpm_frame(
            (440.0, 0.95, -12.0),
            None,
            1024,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert!(return_frame.is_empty(), "Must remain on A4");
        assert_eq!(segmenter.active_mono_note().unwrap().pitch, Pitch::A);

        // 4. Real legato to B4 (493.88 Hz): Frame 1 at ts=1100 (after 100ms sustain >= 50ms)
        let leg1 = segmenter.process_mpm_frame(
            (493.88, 0.95, -12.0),
            None,
            1100,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert!(leg1.is_empty(), "Legato frame 1 pending confirmation");

        // Frame 2 at ts=1112 confirms legato
        let leg2 = segmenter.process_mpm_frame(
            (493.88, 0.95, -12.0),
            None,
            1112,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert_eq!(leg2.len(), 2, "Legato transition confirmed on frame 2");
        assert!(matches!(leg2[0], SegmentedNoteEvent::End(_)));
        assert!(matches!(leg2[1], SegmentedNoteEvent::Start(_)));
        assert_eq!(segmenter.active_mono_note().unwrap().pitch, Pitch::B);
    }

    #[test]
    fn test_mpm_hfc_restrike_debounce() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // 1. Start note at ts=1000
        let _ = segmenter.process_mpm_frame(
            (440.0, 0.95, -12.0),
            None,
            1000,
            12.0,
            -60.0,
            1500.0,
            &profile,
        );

        // 2. Spurious HFC onset 15ms after note start (ts=1015) -> Rebuffed by 80ms debounce
        let early_onset = segmenter.process_mpm_frame(
            (440.0, 0.95, -11.0),
            Some(1015),
            1015,
            14.0,
            -60.0,
            1800.0,
            &profile,
        );
        assert!(
            early_onset.is_empty(),
            "Onset within 80ms must not restrike"
        );

        // 3. Valid HFC onset at ts=1090 (90ms after start >= 80ms) -> Restrikes
        let valid_onset = segmenter.process_mpm_frame(
            (440.0, 0.95, -10.0),
            Some(1090),
            1090,
            14.0,
            -60.0,
            2000.0,
            &profile,
        );
        assert_eq!(
            valid_onset.len(),
            2,
            "Restrike after 80ms must emit End + Start"
        );
    }

    #[test]
    fn test_mpm_elevated_clarity_rejection() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();

        // Low clarity frame (0.65 < 0.80 mono entry clarity) from idle -> No note started
        let noise = segmenter.process_mpm_frame(
            (440.0, 0.65, -15.0),
            None,
            1000,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert!(noise.is_empty());
        assert!(segmenter.active_mono_note().is_none());

        // High clarity frame (0.85 >= 0.80) -> Starts note
        let valid = segmenter.process_mpm_frame(
            (440.0, 0.85, -15.0),
            None,
            1012,
            10.0,
            -60.0,
            1500.0,
            &profile,
        );
        assert_eq!(valid.len(), 1);
        assert!(matches!(valid[0], SegmentedNoteEvent::Start(_)));
    }

    #[test]
    fn test_bytedance_crnn_segmentation_with_offsets_and_pedal() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();
        let frame = [0.0f32; FRAME_SIZE];

        // 1. Note Onset for A4 (MIDI 69 -> bin 48)
        let mut out = ByteDanceCrnnOutput::default();
        out.onsets[48] = 0.85;
        out.frames[48] = 0.90;
        out.velocity[48] = 95.0; // Fortissimo
        out.pedal_frame = 0.80; // Sustain pedal pressed down

        let events = segmenter
            .process_bytedance_crnn_frame(&out, 1000, 8.0, -55.0, 1200.0, &profile, &frame);
        assert_eq!(events.len(), 1);
        if let SegmentedNoteEvent::Start(s) = &events[0] {
            assert_eq!(s.pitch, Pitch::A);
            assert_eq!(s.octave, Octave::O4);
            assert_eq!(s.velocity, Some(95));
            assert_eq!(s.dynamic, DynamicLevel::MezzoForte);
        } else {
            panic!("Expected StartNote event");
        }

        // 2. Key released at ts=1100ms while pedal held -> Musician lets go!
        // EndNote must be emitted immediately when the musician lets go (100ms duration),
        // with DampingProfile::PedalSustained, and marked in pedal_alive array.
        let mut out_pedal = ByteDanceCrnnOutput::default();
        out_pedal.frames[48] = 0.30; // frame dropped below threshold (musician released key)
        out_pedal.pedal_frame = 0.85; // pedal still held

        let events_pedal = segmenter
            .process_bytedance_crnn_frame(&out_pedal, 1100, 4.0, -65.0, 800.0, &profile, &frame);
        assert_eq!(
            events_pedal.len(),
            1,
            "EndNote must be emitted immediately when the musician lets go"
        );
        if let SegmentedNoteEvent::End(e) = &events_pedal[0] {
            assert_eq!(e.pitch, Pitch::A);
            assert_eq!(e.octave, Octave::O4);
            assert_eq!(e.velocity, Some(95));
            assert_eq!(e.dynamic, DynamicLevel::MezzoForte);
            assert_eq!(e.damping, DampingProfile::PedalSustained);
            assert!((e.note_duration - 0.100).abs() < 0.001);
        } else {
            panic!("Expected EndNote event on key release");
        }
        assert!(
            segmenter.bytedance_pedal_alive()[69],
            "A4 must be marked in pedal_alive array"
        );

        // 3. String continues ringing under pedal: high frame activation without onset is IGNORED!
        let mut out_ringing = ByteDanceCrnnOutput::default();
        out_ringing.frames[48] = 0.85; // String still ringing loudly
        out_ringing.pedal_frame = 0.85; // Pedal still down
        out_ringing.energy_dbfs = -30.0;

        let events_ringing = segmenter.process_bytedance_crnn_frame(
            &out_ringing,
            1150,
            6.0,
            -30.0,
            1200.0,
            &profile,
            &frame,
        );
        assert!(
            events_ringing.is_empty(),
            "Ringing strings in pedal_alive array must be ignored without new onset"
        );

        // 4. Pedal is released at ts=1250ms -> pedal_alive is cleared
        let mut out_pedal_off = ByteDanceCrnnOutput::default();
        out_pedal_off.pedal_frame = 0.10; // Pedal released
        let events_off = segmenter.process_bytedance_crnn_frame(
            &out_pedal_off,
            1250,
            2.0,
            -70.0,
            500.0,
            &profile,
            &frame,
        );
        assert!(events_off.is_empty());
        assert!(
            !segmenter.bytedance_pedal_alive()[69],
            "pedal_alive must be cleared on pedal release"
        );
    }

    #[test]
    fn test_bytedance_crnn_pedal_alive_restrike() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();
        let frame = [0.0f32; FRAME_SIZE];

        // 1. Play A4 with pedal held down at ts=1000ms
        let mut out1 = ByteDanceCrnnOutput::default();
        out1.onsets[48] = 0.85;
        out1.frames[48] = 0.90;
        out1.velocity[48] = 75.0;
        out1.pedal_frame = 0.85;

        let ev1 = segmenter
            .process_bytedance_crnn_frame(&out1, 1000, 8.0, -50.0, 1000.0, &profile, &frame);
        assert_eq!(ev1.len(), 1);
        assert!(matches!(ev1[0], SegmentedNoteEvent::Start(_)));

        // Advance frames through sustain (1020, 1040, 1060, 1080) to expire the 80ms restrike lockout
        for t in [1020, 1040, 1060, 1080] {
            let mut out_sustain = ByteDanceCrnnOutput::default();
            out_sustain.frames[48] = 0.85;
            out_sustain.pedal_frame = 0.85;
            let _ = segmenter.process_bytedance_crnn_frame(
                &out_sustain,
                t,
                6.0,
                -50.0,
                900.0,
                &profile,
                &frame,
            );
        }

        // 2. Musician lets go of key at ts=1100ms -> EndNote emitted, key enters pedal_alive
        let mut out_release = ByteDanceCrnnOutput::default();
        out_release.frames[48] = 0.20;
        out_release.pedal_frame = 0.85;

        let ev2 = segmenter.process_bytedance_crnn_frame(
            &out_release,
            1100,
            4.0,
            -60.0,
            800.0,
            &profile,
            &frame,
        );
        assert_eq!(ev2.len(), 1);
        assert!(matches!(ev2[0], SegmentedNoteEvent::End(_)));
        assert!(segmenter.bytedance_pedal_alive()[69]);

        // 3. String ringing under pedal is ignored
        let mut out_ring = ByteDanceCrnnOutput::default();
        out_ring.frames[48] = 0.80;
        out_ring.pedal_frame = 0.85;
        let ev3 = segmenter.process_bytedance_crnn_frame(
            &out_ring,
            1120,
            5.0,
            -52.0,
            800.0,
            &profile,
            &frame,
        );
        assert!(ev3.is_empty());

        // 4. Musician restrikes A4 at ts=1140ms (new onset spike detected!)
        let mut out_restrike = ByteDanceCrnnOutput::default();
        out_restrike.onsets[48] = 0.92;
        out_restrike.frames[48] = 0.95;
        out_restrike.velocity[48] = 110.0;
        out_restrike.pedal_frame = 0.85;

        let ev4 = segmenter.process_bytedance_crnn_frame(
            &out_restrike,
            1140,
            9.0,
            -45.0,
            1200.0,
            &profile,
            &frame,
        );
        assert_eq!(ev4.len(), 1, "New onset must start note even if in pedal_alive");
        if let SegmentedNoteEvent::Start(s) = &ev4[0] {
            assert_eq!(s.pitch, Pitch::A);
            assert_eq!(s.octave, Octave::O4);
            assert_eq!(s.velocity, Some(110));
            assert_eq!(s.dynamic, DynamicLevel::Forte);
        } else {
            panic!("Expected StartNote event on restrike");
        }
        assert!(
            !segmenter.bytedance_pedal_alive()[69],
            "pedal_alive must be cleared once new onset is struck"
        );
    }

    #[test]
    fn test_bytedance_crnn_restrike_debouncing() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();
        let frame = [0.0f32; FRAME_SIZE];

        // 1. Initial strike for C4 (MIDI 60 -> bin 39)
        let mut out = ByteDanceCrnnOutput::default();
        out.onsets[39] = 0.80;
        out.frames[39] = 0.85;
        out.velocity[39] = 80.0; // MezzoForte

        let events = segmenter
            .process_bytedance_crnn_frame(&out, 1000, 6.0, -50.0, 1000.0, &profile, &frame);
        assert_eq!(events.len(), 1);
        if let SegmentedNoteEvent::Start(s) = &events[0] {
            assert_eq!(s.velocity, Some(80));
        }

        // 2. Immediate next frame (20ms later): onset remains high (smooth regression curve)
        // Restrike lockout should debounce and prevent premature note termination
        let mut out2 = ByteDanceCrnnOutput::default();
        out2.onsets[39] = 0.82;
        out2.frames[39] = 0.85;
        out2.velocity[39] = 82.0;

        let events2 = segmenter
            .process_bytedance_crnn_frame(&out2, 1020, 6.0, -50.0, 1000.0, &profile, &frame);
        assert!(
            events2.is_empty(),
            "Consecutive onset frames within lockout must be debounced"
        );

        // Advance 3 more frames (60ms) to expire the 4-frame lockout
        for t in [1040, 1060, 1080] {
            let mut out_sustain = ByteDanceCrnnOutput::default();
            out_sustain.frames[39] = 0.70;
            let _ = segmenter.process_bytedance_crnn_frame(
                &out_sustain,
                t,
                5.0,
                -52.0,
                950.0,
                &profile,
                &frame,
            );
        }

        // 3. Genuine physical restrike after lockout expires (at 100ms) with higher velocity
        let mut out_restrike = ByteDanceCrnnOutput::default();
        out_restrike.onsets[39] = 0.90;
        out_restrike.frames[39] = 0.88;
        out_restrike.velocity[39] = 105.0; // Forte

        let events_restrike = segmenter.process_bytedance_crnn_frame(
            &out_restrike,
            1100,
            8.0,
            -45.0,
            1200.0,
            &profile,
            &frame,
        );
        assert_eq!(
            events_restrike.len(),
            2,
            "Expected EndNote for previous strike and StartNote for new strike"
        );
        if let SegmentedNoteEvent::End(e) = &events_restrike[0] {
            assert_eq!(
                e.velocity, Some(80),
                "Previous note must retain its original strike velocity"
            );
        }
        if let SegmentedNoteEvent::Start(s) = &events_restrike[1] {
            assert_eq!(
                s.velocity, Some(105),
                "New note must receive the restrike velocity"
            );
            assert_eq!(s.dynamic, DynamicLevel::Forte);
        }
    }

    #[test]
    fn test_bytedance_crnn_ghost_note_rejection() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();
        let frame = [0.0f32; FRAME_SIZE];

        // Moderate frame probability (e.g. 0.45 from harmonic resonance), but onset is 0.0
        let mut out = ByteDanceCrnnOutput::default();
        out.frames[39] = 0.45; // > frame_threshold (0.40) but < LEGATO_FRAME_THRESHOLD (0.65)
        out.onsets[39] = 0.05; // No onset spike

        let events =
            segmenter.process_bytedance_crnn_frame(&out, 1000, 4.0, -60.0, 800.0, &profile, &frame);
        assert!(
            events.is_empty(),
            "Overtone harmonic resonance without onset or high legato frame must be rejected"
        );
    }

    #[test]
    fn test_bytedance_crnn_finalize_all_preserves_velocity() {
        let mut segmenter = StreamingNoteSegmenter::new(0.5, 0.4, 44100);
        let profile = Instrument::Piano.acoustic_profile();
        let frame = [0.0f32; FRAME_SIZE];

        // Start note G4 (MIDI 67 -> bin 46) with velocity 115 (Fortissimo)
        let mut out = ByteDanceCrnnOutput::default();
        out.onsets[46] = 0.85;
        out.frames[46] = 0.90;
        out.velocity[46] = 115.0;

        let _ = segmenter
            .process_bytedance_crnn_frame(&out, 1000, 9.0, -40.0, 1500.0, &profile, &frame);

        // Stream suddenly ends at 1200ms -> finalize_all
        let finalized = segmenter.finalize_all(1200, &profile);
        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].pitch, Pitch::G);
        assert_eq!(finalized[0].octave, Octave::O4);
        assert_eq!(
            finalized[0].velocity, Some(115),
            "Finalize all must preserve the neural velocity"
        );
        assert_eq!(finalized[0].dynamic, DynamicLevel::Fortissimo);
    }
}
