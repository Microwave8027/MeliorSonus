use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::processing::functions::mpm::MPM;
use crate::audio_processing::processing::functions::rms_dbfs::loudness;
use crate::constants::*;
use crate::utils::feature_extractor_state::{track_note_state, NoteEnvelopeState};
use arrayvec::ArrayVec;
use cpal::StreamConfig;

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

    pub fn finalize_note(&mut self) -> Option<EndNote> {
        self.active_note.take().map(|active| active.into_end_note())
    }

    pub fn reset(&mut self) -> Option<EndNote> {
        let finalized = self.finalize_note();
        self.state = FeatureExtractorState::Idle;
        self.active_note = None;
        finalized
    }

    pub fn take_active_note(&mut self) -> Option<(RecordNote, FeatureExtractorState)> {
        let state = self.state;
        self.state = FeatureExtractorState::Idle;
        self.active_note.take().map(|record| (record, state))
    }

    pub fn adopt_note(&mut self, note: RecordNote, state: FeatureExtractorState) {
        self.active_note = Some(note);
        self.state = state;
    }

    pub fn processing_single_note(
        &mut self,
        filtered_frame: &[f32; FRAME_SIZE],
        cfg: &StreamConfig,
        instrument: &Instrument,
        mpm: &mut MPM,
        time_stamp: u128,
    ) -> ArrayVec<Notes, 2> {
        let dbfs = loudness(filtered_frame);
        let frequency = mpm.mpm(filtered_frame, cfg, instrument);

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
            |_| {},
        )
    }
}
