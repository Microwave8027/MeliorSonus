/*
 * Polyphonic Feature Extractor & Harmonic Sieve Architecture
 *
 * Provides dual-path real-time audio analysis:
 * 1. Fast-Path Monophonic: Bypasses CRNN (CRNN Sleep Mode = 0% CPU) when NSDF clarity is high.
 *    Delegates to `NoteFeatureExtractorImpl` using McLeod Pitch Method (MPM).
 * 2. Polyphonic CRNN + Harmonic Sieve Path: Triggered when NSDF clarity drops below threshold
 *    or secondary peak ratio indicates multiple active fundamental frequencies.
 *    Applies Log-CQT binning and Harmonic Sieve comb filtering to disentangle ghost harmonics.
 *
 * Preserves 100% of existing `RecordNote` envelope tracking (Instant::now(), rise_time, note_duration, peak_dbfs).
 */

use crate::constants::*;
use crate::harmonic_sieve_mask::HarmonicSieveMasker;
use crate::notes::*;
use crate::processor::{NoteEnvelopeState, track_note_state};
use arrayvec::ArrayVec;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PolyphonyMode {
    Silence,
    SingleNoteFastPath,
    PolyphonicCrnnPath,
    PolyphonicHangover(u8),
}

pub type PolyphonicNoteState = NoteEnvelopeState;

/// Polyphonic Feature Extractor with zero dynamic allocation
pub struct PolyphonicFeatureExtractorImpl {
    pub sieve: HarmonicSieveMasker,
    pub poly_active_notes: [Option<RecordNote>; MAX_POLYPHONY],
    pub poly_note_states: [PolyphonicNoteState; MAX_POLYPHONY],
    pub crnn_pitch_probs: [f32; PITCH_BINS],
    pub sieved_pitch_probs: [f32; PITCH_BINS],
}

impl PolyphonicFeatureExtractorImpl {
    pub fn new() -> Self {
        Self {
            sieve: HarmonicSieveMasker::new(0.0002, 6),
            poly_active_notes: Default::default(),
            poly_note_states: Default::default(),
            crnn_pitch_probs: [0.0f32; PITCH_BINS],
            sieved_pitch_probs: [0.0f32; PITCH_BINS],
        }
    }

    /// Polyphonic path execution using CRNN + Harmonic Sieve
    pub fn process_polyphonic_path(
        &mut self,
        buffer: &[f32; FRAME_SIZE],
        sample_rate: f32,
        timestamp: u128,
        dbfs: f32,
    ) -> ArrayVec<Note, MAX_POLYPHONY> {
        Self::run_tract_inference(buffer, &mut self.crnn_pitch_probs);

        self.sieve.apply_sieve(
            &self.crnn_pitch_probs,
            sample_rate,
            &mut self.sieved_pitch_probs,
        );

        let mut finalized_notes = ArrayVec::new();

        // Update Multi-Note State Machine across pre-allocated slots
        for bin in 0..PITCH_BINS {
            let prob = self.sieved_pitch_probs[bin];
            let midi = (bin + 21) as u8;
            let pitch = Pitch::get_pitch(midi);
            let octave = Octave::midi_to_note(midi);

            if prob >= 0.50 {
                // Find existing active slot for this pitch/octave
                let mut slot_idx = None;
                for (i, slot) in self.poly_active_notes.iter().enumerate() {
                    if slot
                        .as_ref()
                        .is_some_and(|r| r.pitch == pitch && r.octave == octave)
                    {
                        slot_idx = Some(i);
                        break;
                    }
                }

                // If not found, find an empty / idle slot
                if slot_idx.is_none() {
                    for (i, slot) in self.poly_active_notes.iter().enumerate() {
                        if slot.is_none() {
                            slot_idx = Some(i);
                            break;
                        }
                    }
                }

                if let Some(i) = slot_idx {
                    let mut env_state: NoteEnvelopeState = self.poly_note_states[i];
                    if let Some(note) = track_note_state(
                        &mut env_state,
                        &mut self.poly_active_notes[i],
                        Some((pitch, octave, 0)),
                        dbfs,
                        timestamp,
                        |_| {},
                        |_, _| {},
                        |_| {},
                    ) {
                        finalized_notes.push(note);
                    }
                    self.poly_note_states[i] = env_state;
                }
            } else {
                // Inactive: if this pitch was active, transition to None / Idle
                for i in 0..MAX_POLYPHONY {
                    let is_match = self.poly_active_notes[i]
                        .as_ref()
                        .is_some_and(|r| r.pitch == pitch && r.octave == octave);
                    if is_match {
                        let mut env_state: NoteEnvelopeState = self.poly_note_states[i];
                        if let Some(note) = track_note_state(
                            &mut env_state,
                            &mut self.poly_active_notes[i],
                            None,
                            dbfs,
                            timestamp,
                            |_| {},
                            |_, _| {},
                            |_| {},
                        ) {
                            finalized_notes.push(note);
                        }
                        self.poly_note_states[i] = env_state;
                    }
                }
            }
        }

        finalized_notes
    }

    /// Zero-allocation placeholder for ONNX runtime inference via tract-onnx SimplePlan
    fn run_tract_inference(_buffer: &[f32; FRAME_SIZE], out_probs: &mut [f32; PITCH_BINS]) {
        out_probs.fill(0.0);
    }

    pub fn adopt_note(&mut self, note: RecordNote, state: PolyphonicNoteState) {
        // Check if there is an existing active slot for this pitch/octave
        for (i, slot) in self.poly_active_notes.iter_mut().enumerate() {
            if let Some(existing) = slot
                .as_mut()
                .filter(|e| e.pitch == note.pitch && e.octave == note.octave)
            {
                if note.beginning < existing.beginning {
                    existing.beginning = note.beginning;
                    existing.note_striked = note.note_striked;
                }
                existing.add_peak_dbfs(note.peak_dbfs);
                self.poly_note_states[i] = state;
                return;
            }
        }

        // Otherwise find first free slot
        for (i, slot) in self.poly_active_notes.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(note);
                self.poly_note_states[i] = state;
                return;
            }
        }
    }

    pub fn take_primary_active_note_and_finalize_rest<F>(
        &mut self,
        mut on_finalize: F,
    ) -> Option<(RecordNote, PolyphonicNoteState)>
    where
        F: FnMut(Note),
    {
        // Find the loudest active note to adopt as primary
        let mut primary_idx = None;
        let mut highest_dbfs = f32::NEG_INFINITY;
        for (i, slot) in self.poly_active_notes.iter().enumerate() {
            if let Some(record) = slot.as_ref().filter(|r| r.peak_dbfs > highest_dbfs) {
                highest_dbfs = record.peak_dbfs;
                primary_idx = Some(i);
            }
        }

        let primary = if let Some(idx) = primary_idx {
            let note = self.poly_active_notes[idx].take();
            let state = self.poly_note_states[idx];
            self.poly_note_states[idx] = PolyphonicNoteState::Idle;
            note.map(|n| (n, state))
        } else {
            None
        };

        // Finalize all remaining active notes
        for i in 0..MAX_POLYPHONY {
            if let Some(active) = self.poly_active_notes[i].take() {
                self.poly_note_states[i] = PolyphonicNoteState::Idle;
                let note = active.into_note();
                on_finalize(note);
            }
        }

        primary
    }
}

impl Default for PolyphonicFeatureExtractorImpl {
    fn default() -> Self {
        Self::new()
    }
}
