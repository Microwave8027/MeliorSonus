/*
[ Polyphonic Audio Mixture ]
                              │
                              ▼
           [ Multi-Pitch Transcriber (CRNN / AMT) ]
                              │
            Outputs Active Pitches ($f_0^{(i)}$) & Timestamps
                              │
                              ▼
             [ Harmonic Sieve / Masking Layer ]
             (Removes collided harmonic peaks)
            ┌─────────────────┴─────────────────┐
            ▼                                   ▼
    [ Voice A Slices ]                  [ Voice B Slices ]
            │                                   │
 ┌──────────┴──────────┐             ┌──────────┴──────────┐
 ▼                     ▼             ▼                     ▼
[Attack & ADSR]   [HNR & Timbre]   [Attack & ADSR]   [HNR & Timbre]
* TODO
* Currently no algorithms requiring ffi are implemented. A basic mpm and rms dbfs are used for calculating pitch, tonality, loudness, and a duration is returned which can later be calculated.
* Look in dsp.rs for workflow
* The DspCallback currently only calls the single note feature extractor, no harmonic sieve or crnn for serperation yet.
* Mpm will be removed if a crnn processes it
* the crnn will go to sleep if a algorithm decides its not that complicated.
 */
use crate::constants::*;
use crate::high_pass_filter::BandPassFilter;
use crate::instruments::Instrument;
use crate::mpm::MPM;
use crate::notes::*;
use crate::prelude::*;
use crate::processor::{track_note_state, NoteEnvelopeState};
use crate::rms_dbfs::*;

pub type FeatureExtractorState = NoteEnvelopeState;

pub struct NoteFeatureExtractorImpl {
    state: FeatureExtractorState,
    active_note: Option<RecordNote>,
    thresh_hold: f32,
}

impl NoteFeatureExtractorImpl {
    pub fn new(thresh_hold: f32) -> Self {
        Self {
            state: FeatureExtractorState::Idle,
            active_note: None,
            thresh_hold,
        }
    }

    pub fn state(&self) -> FeatureExtractorState {
        self.state
    }

    pub fn active_note(&self) -> Option<&RecordNote> {
        self.active_note.as_ref()
    }

    pub fn finalize_note(&mut self, time_stamp: u128) -> Option<(Note, u128)> {
        self.active_note.take().map(|active| (active.into_note(), time_stamp))
    }

    pub fn reset(&mut self, time_stamp: u128) -> Option<(Note, u128)> {
        let finalized = self.finalize_note(time_stamp);
        self.state = FeatureExtractorState::Idle;
        self.active_note = None;
        finalized
    }

    pub fn processing_single_note(
        &mut self,
        buffer: &[f32; FRAME_SIZE],
        cfg: &StreamConfig,
        filter: &mut BandPassFilter,
        instrument: &Instrument,
        mpm: &mut MPM,
        time_stamp: u128,
    ) -> Option<(Note, u128)> {
        let filtered_frame = filter.process_frames(buffer);
        let dbfs = loudness(filtered_frame.as_slice());
        let frequency = mpm.mpm(&filtered_frame, cfg, instrument);

        let detected = if dbfs >= self.thresh_hold {
            get_note(frequency)
        } else {
            None
        };

        track_note_state(
            &mut self.state,
            &mut self.active_note,
            detected,
            dbfs,
            time_stamp,
            |_| {},
            |_, _| {},
            |_, _| {},
        )
    }
}
