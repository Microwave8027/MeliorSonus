/*
* Feature extractor state for tracking notes in the feature extractor, specifically duration.
* This process tracks each note for both polyphonic and monophonic.
* Functions for returing values note values are passed into this function
*/
use crate::notes::{Note, Octave, Pitch, RecordNote};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum NoteEnvelopeState {
    #[default]
    Idle,
    Rise,
    Peak,
    Decay,
}

/// Stateless note tracking processor that mutates borrowed note state and active note.
/// Peak is currently deprecated and not used
/// Implements envelope transitions (Idle -> Rise -> Decay -> Finalize)
/// while allowing monophonic and polyphonic pipelines to customize behavior via closures.
#[allow(clippy::too_many_arguments)]
pub fn track_note_state<FStart, FUpdate, FFinalize>(
    state: &mut NoteEnvelopeState,
    active_note: &mut Option<RecordNote>,
    detected: Option<(Pitch, Octave, i8)>,
    dbfs: f32,
    time_stamp: u128,
    mut on_start: FStart,
    mut on_update: FUpdate,
    mut on_finalize: FFinalize,
) -> Option<Note>
where
    FStart: FnMut(&mut RecordNote),
    FUpdate: FnMut(&mut RecordNote, NoteEnvelopeState),
    FFinalize: FnMut(&Note),
{
    let mut finalized = |note: &mut Option<RecordNote>| {
        note.take().map(|note| {
            let note = note.into_note();
            on_finalize(&note);
            note
        })
    };
    let result = match (*state, detected) {
        (NoteEnvelopeState::Idle, Some((pitch, octave, tonality_offset))) => {
            let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
            on_start(&mut record);
            *active_note = Some(record);
            *state = NoteEnvelopeState::Rise;
            None
        }
        (NoteEnvelopeState::Idle, None) => None,
        (NoteEnvelopeState::Rise, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    active.tonality_offset = tonality_offset;
                    if dbfs >= active.peak_dbfs {
                        active.add_peak_dbfs(dbfs);
                        on_update(active, NoteEnvelopeState::Rise);
                    } else {
                        active.record_rise_time();
                        *state = NoteEnvelopeState::Decay;
                        on_update(active, NoteEnvelopeState::Decay);
                    }
                    None
                } else {
                    let finalized = finalized(active_note);
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    finalized
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                None
            }
        }
        (NoteEnvelopeState::Rise, None) => {
            let finalized = finalized(active_note);
            *state = NoteEnvelopeState::Idle;
            finalized
        }
        (NoteEnvelopeState::Peak, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    active.record_rise_time();
                    active.tonality_offset = tonality_offset;
                    *state = NoteEnvelopeState::Decay;
                    on_update(active, NoteEnvelopeState::Decay);
                    None
                } else {
                    let finalized = finalized(active_note);
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    finalized
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                None
            }
        }
        (NoteEnvelopeState::Peak, None) => {
            let finalized = finalized(active_note);
            *state = NoteEnvelopeState::Idle;
            finalized
        }
        (NoteEnvelopeState::Decay, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    if dbfs > active.last_dbfs + 3.0 {
                        let finalized = finalized(active_note);
                        let mut record =
                            RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                        on_start(&mut record);
                        *active_note = Some(record);
                        *state = NoteEnvelopeState::Rise;
                        finalized
                    } else {
                        active.tonality_offset = tonality_offset;
                        on_update(active, NoteEnvelopeState::Decay);
                        None
                    }
                } else {
                    let finalized = finalized(active_note);
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    finalized
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                None
            }
        }
        (NoteEnvelopeState::Decay, None) => {
            let finalized = finalized(active_note);
            *state = NoteEnvelopeState::Idle;
            finalized
        }
    };

    if let Some(active) = active_note {
        active.last_dbfs = dbfs;
    }

    result
}
