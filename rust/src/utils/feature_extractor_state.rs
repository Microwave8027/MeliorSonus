/*
 * Feature extractor state for tracking notes in the feature extractor, specifically duration.
 * This process tracks each note for both polyphonic and monophonic.
 * Functions for returning note values are passed into this function.
 */
use crate::audio_processing::instruments::instrument::InstrumentAcousticProfile;
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
    crest_factor: f32,
    sub_thump_dbfs: f32,
    spectral_centroid: f32,
    mpm_clarity: Option<f32>,
    profile: &InstrumentAcousticProfile,
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
    let mut finalized = |note: &mut Option<RecordNote>, end_ts: u128, is_legato: bool| {
        note.take().map(|note| {
            let end_note = note.into_end_note_with_profile(end_ts, is_legato, profile);
            on_finalize(&end_note);
            end_note
        })
    };

    match (*state, detected) {
        (NoteEnvelopeState::Idle, Some((pitch, octave, tonality_offset))) => {
            let mut record = RecordNote::new_with_features(
                pitch,
                octave,
                tonality_offset,
                dbfs,
                time_stamp,
                crest_factor,
                sub_thump_dbfs,
                spectral_centroid,
                mpm_clarity,
                None,
                None,
            );
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
                    active.last_dbfs = dbfs;
                    if mpm_clarity.is_some() {
                        active.mpm_clarity = mpm_clarity;
                    }
                    active.accumulate_frame(tonality_offset, spectral_centroid, None);
                    if dbfs >= active.peak_dbfs {
                        active.add_peak_dbfs(dbfs);
                        on_update(active, NoteEnvelopeState::Rise);
                    } else {
                        active.record_rise_time(time_stamp);
                        *state = NoteEnvelopeState::Decay;
                        on_update(active, NoteEnvelopeState::Decay);
                    }
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp, true) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        dbfs,
                        time_stamp,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        mpm_clarity,
                        None,
                        None,
                    );
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new_with_features(
                    pitch,
                    octave,
                    tonality_offset,
                    dbfs,
                    time_stamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    mpm_clarity,
                    None,
                    None,
                );
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Rise, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp, false) {
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
                    active.last_dbfs = dbfs;
                    if mpm_clarity.is_some() {
                        active.mpm_clarity = mpm_clarity;
                    }
                    active.accumulate_frame(tonality_offset, spectral_centroid, None);
                    *state = NoteEnvelopeState::Decay;
                    on_update(active, NoteEnvelopeState::Decay);
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp, true) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        dbfs,
                        time_stamp,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        mpm_clarity,
                        None,
                        None,
                    );
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new_with_features(
                    pitch,
                    octave,
                    tonality_offset,
                    dbfs,
                    time_stamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    mpm_clarity,
                    None,
                    None,
                );
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Peak, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp, false) {
                out.push(Notes::End(end_note));
            }
            *state = NoteEnvelopeState::Idle;
        }
        (NoteEnvelopeState::Decay, Some((pitch, octave, tonality_offset))) => {
            if let Some(active) = active_note {
                if active.pitch == pitch && active.octave == octave {
                    // Same frequency re-attack: requires a prior dip and a distinct attack-up
                    let had_prior_dip = active.last_dbfs <= active.peak_dbfs - 2.5;
                    let steep_surge = dbfs >= active.last_dbfs + 4.0;
                    let sharp_transient = crest_factor >= 11.5;
                    let is_reattack = had_prior_dip && steep_surge && sharp_transient;

                    if is_reattack {
                        if let Some(end_note) = finalized(active_note, time_stamp, false) {
                            out.push(Notes::End(end_note));
                        }
                        let mut record = RecordNote::new_with_features(
                            pitch,
                            octave,
                            tonality_offset,
                            dbfs,
                            time_stamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            mpm_clarity,
                            None,
                            None,
                        );
                        let start_note = record.to_start_note();
                        on_start(&mut record);
                        *active_note = Some(record);
                        *state = NoteEnvelopeState::Rise;
                        out.push(Notes::Start(start_note));
                    } else {
                        active.last_dbfs = dbfs;
                        active.tonality_offset = tonality_offset;
                        active.last_timestamp = time_stamp;
                        if mpm_clarity.is_some() {
                            active.mpm_clarity = mpm_clarity;
                        }
                        active.accumulate_frame(tonality_offset, spectral_centroid, None);
                        if dbfs > active.peak_dbfs {
                            active.add_peak_dbfs(dbfs);
                        }
                        on_update(active, NoteEnvelopeState::Decay);
                    }
                } else {
                    if let Some(end_note) = finalized(active_note, time_stamp, true) {
                        out.push(Notes::End(end_note));
                    }
                    let mut record = RecordNote::new_with_features(
                        pitch,
                        octave,
                        tonality_offset,
                        dbfs,
                        time_stamp,
                        crest_factor,
                        sub_thump_dbfs,
                        spectral_centroid,
                        mpm_clarity,
                        None,
                        None,
                    );
                    let start_note = record.to_start_note();
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    out.push(Notes::Start(start_note));
                }
            } else {
                let mut record = RecordNote::new_with_features(
                    pitch,
                    octave,
                    tonality_offset,
                    dbfs,
                    time_stamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    mpm_clarity,
                    None,
                    None,
                );
                let start_note = record.to_start_note();
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                out.push(Notes::Start(start_note));
            }
        }
        (NoteEnvelopeState::Decay, None) => {
            if let Some(end_note) = finalized(active_note, time_stamp, false) {
                out.push(Notes::End(end_note));
            }
            *state = NoteEnvelopeState::Idle;
        }
    }

    out
}
