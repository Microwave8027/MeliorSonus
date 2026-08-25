use crate::constants::*;
use crate::dsp::{CallBackParameters, DspCallBack};
use crate::monophonic_feature_extractor::NoteFeatureExtractorImpl;
use crate::notes::*;
use crate::nsdf::NsdfEvaluator;
use crate::polyphonic_feature_extractor::{
    PolyphonicFeatureExtractorImpl, PolyphonicNoteState, PolyphonyMode,
};
use crate::rms_dbfs::loudness;
use ringbuf::HeapProd;
use ringbuf::traits::Producer;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(non_camel_case_types)]
pub struct dsp_feature_extractor {
    pub note_rb: HeapProd<Note>,
    pub single_note_extractor: NoteFeatureExtractorImpl,
    pub polyphonic_extractor: PolyphonicFeatureExtractorImpl,
    pub mode: PolyphonyMode,
    pub nsdf_evaluator: NsdfEvaluator,
    pub hangover_counter: u8,
    pub silence_threshold_dbfs: f32,
}

pub type DspFeatureExtractor = dsp_feature_extractor;

impl dsp_feature_extractor {
    pub fn new(
        note_rb: HeapProd<Note>,
        single_note_extractor: NoteFeatureExtractorImpl,
        polyphonic_extractor: PolyphonicFeatureExtractorImpl,
        silence_threshold_dbfs: f32,
    ) -> Self {
        Self {
            note_rb,
            single_note_extractor,
            polyphonic_extractor,
            mode: PolyphonyMode::Silence,
            nsdf_evaluator: NsdfEvaluator::new(),
            hangover_counter: HANGOVER_FRAMES_DEFAULT,
            silence_threshold_dbfs,
        }
    }

    pub fn mode(&self) -> PolyphonyMode {
        self.mode
    }
}

impl DspCallBack for dsp_feature_extractor {
    fn dsp_callback(
        &mut self,
        CallBackParameters {
            buffer,
            cfg,
            filter,
            instrument,
            mpm,
        }: CallBackParameters,
    ) {
        let timestamp: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let filtered_frame = filter.process_frames(buffer);
        let dbfs = loudness(filtered_frame.as_slice());

        if dbfs < self.silence_threshold_dbfs {
            // Finalize & push all active polyphonic notes to ringbuffer
            for i in 0..MAX_POLYPHONY {
                if let Some(active) = self.polyphonic_extractor.poly_active_notes[i].take() {
                    self.polyphonic_extractor.poly_note_states[i] = PolyphonicNoteState::Idle;
                    let note = active.into_note();
                    let _ = self.note_rb.try_push(note);
                }
            }

            if let Some(note) = self.single_note_extractor.processing_single_note(
                &filtered_frame,
                cfg,
                instrument,
                mpm,
                timestamp,
            ) {
                let _ = self.note_rb.try_push(note);
            }

            self.mode = PolyphonyMode::Silence;
            self.hangover_counter = 0;
            return;
        }

        let (clarity, peak_ratio) = self.nsdf_evaluator.evaluate_frame(&filtered_frame);

        let mpm_cfg = instrument.mpm_config();
        let monophonic_threshold = mpm_cfg.clarity_threshold;
        let is_monophonic = clarity >= monophonic_threshold && peak_ratio <= 0.55;

        let prev_mode = self.mode;

        if is_monophonic {
            if self.mode == PolyphonyMode::Silence || self.hangover_counter == 0 {
                self.mode = PolyphonyMode::SingleNoteFastPath;
                self.hangover_counter = 0;
            } else {
                self.hangover_counter -= 1;
                self.mode = PolyphonyMode::PolyphonicHangover(self.hangover_counter);
            }
        } else {
            self.mode = PolyphonyMode::PolyphonicCrnnPath;
            self.hangover_counter = HANGOVER_FRAMES_DEFAULT;
        }

        // Migrate active notes across mode transitions to preserve note continuity
        match (prev_mode, self.mode) {
            (PolyphonyMode::SingleNoteFastPath, PolyphonyMode::PolyphonicCrnnPath)
            | (PolyphonyMode::SingleNoteFastPath, PolyphonyMode::PolyphonicHangover(_)) => {
                if let Some((active, state)) = self.single_note_extractor.take_active_note() {
                    self.polyphonic_extractor.adopt_note(active, state);
                }
            }

            (PolyphonyMode::PolyphonicCrnnPath, PolyphonyMode::SingleNoteFastPath)
            | (PolyphonyMode::PolyphonicHangover(_), PolyphonyMode::SingleNoteFastPath) => {
                if let Some((primary, state)) = self
                    .polyphonic_extractor
                    .take_primary_active_note_and_finalize_rest(|finalized| {
                        let _ = self.note_rb.try_push(finalized);
                    })
                {
                    self.single_note_extractor.adopt_note(primary, state);
                }
            }

            _ => {}
        }

        match self.mode {
            PolyphonyMode::Silence => {}

            PolyphonyMode::SingleNoteFastPath => {
                // Delegate to MPM Fast Path (0 CRNN Overhead)
                if let Some(note) = self.single_note_extractor.processing_single_note(
                    &filtered_frame,
                    cfg,
                    instrument,
                    mpm,
                    timestamp,
                ) {
                    let _ = self.note_rb.try_push(note);
                }
            }

            PolyphonyMode::PolyphonicCrnnPath | PolyphonyMode::PolyphonicHangover(_) => {
                // Route to Polyphonic CRNN + Harmonic Sieve
                for note in self.polyphonic_extractor.process_polyphonic_path(
                    &filtered_frame,
                    cfg.sample_rate as f32,
                    timestamp,
                    dbfs,
                ) {
                    let _ = self.note_rb.try_push(note);
                }
            }
        }
    }
}
