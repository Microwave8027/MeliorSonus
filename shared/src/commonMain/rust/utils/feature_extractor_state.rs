/*
 * Feature extractor state for tracking notes in the feature extractor, specifically duration.
 * This process tracks each note for both polyphonic and monophonic.
 * Functions for returning note values are passed into this function.
 */
use crate::audio_processing::instruments::notes::{EndNote, Notes, Octave, Pitch, RecordNote};
use arrayvec::ArrayVec;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum NoteEnvelopeState {
    #[default]
    Idle,
    Rise,
    Peak,
    Decay,
}

/// Stateless note tracking processor that mutates borrowed note state and active note.
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
) -> ArrayVec<Notes, 2>
where
    FStart: FnMut(&mut RecordNote),
    FUpdate: FnMut(&mut RecordNote, NoteEnvelopeState),
    FFinalize: FnMut(&EndNote),
{
    let mut out = ArrayVec::new();
    let mut finalized = |note: &mut Option<RecordNote>, end_ts: u128| {
        note.take().map(|note| {
            let end_note = note.into_end_note_with_timestamp(end_ts);
            on_finalize(&end_note);
            end_note
        })
    };

    match (*state, detected) {
        (NoteEnvelopeState::Idle, Some((pitch, octave, tonality_offset))) => {
            let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
            let start_note = record.to_start_note();
            on_start(&mut record);
            *active_note = Some(record);
            *state = NoteEnvelopeState::Rise;
            out.push(Notes::Start(start_note));
        }
        (NoteEnvelopeState::Idle, None) => {}
        (NoteEnvelopeState::Rise, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    active.tonality_offset = tonality_offset;
                    active.last_timestamp = time_stamp;
                    if dbfs >= active.peak_dbfs {
                        active.add_peak_dbfs(dbfs);
                        on_update(active, NoteEnvelopeState::Rise);
                    } else {
                        active.record_rise_time(time_stamp);
                        *state = NoteEnvelopeState::Decay;
                        on_update(active, NoteEnvelopeState::Decay);
                    }
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Rise, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp) {
                out.push(Notes::End(end_note));
            }
            *state = NoteEnvelopeState::Idle;
        }
        (NoteEnvelopeState::Peak, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    active.record_rise_time(time_stamp);
                    active.tonality_offset = tonality_offset;
                    active.last_timestamp = time_stamp;
                    *state = NoteEnvelopeState::Decay;
                    on_update(active, NoteEnvelopeState::Decay);
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Peak, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp) {
                out.push(Notes::End(end_note));
            }
            *state = NoteEnvelopeState::Idle;
        }
        (NoteEnvelopeState::Decay, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    if dbfs > active.last_dbfs + 3.0 {
                        if let Some(end_note) = finalized(active_note, time_stamp) {
                            out.push(Notes::End(end_note));
                        }
                        let mut record =
                            RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                        let start_note = record.to_start_note();
                        on_start(&mut record);
                        *active_note = Some(record);
                        *state = NoteEnvelopeState::Rise;
                        out.push(Notes::Start(start_note));
                    } else {
                        active.tonality_offset = tonality_offset;
                        active.last_timestamp = time_stamp;
                        on_update(active, NoteEnvelopeState::Decay);
                    }
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record =
                        RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs, time_stamp);
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Decay, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp) {
                out.push(Notes::End(end_note));
            }
            *state = NoteEnvelopeState::Idle;
        }
    };

    if let Some(active) = active_note {
        active.last_dbfs = dbfs;
    }

    out
}
