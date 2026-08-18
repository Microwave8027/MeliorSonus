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
use crate::notes::*;
use crate::prelude::*;
use crate::rms_dbfs::*;
use ringbuf::HeapProd;
use ringbuf::traits::Producer;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NoteFeatureExtractorImpl {
    state: FeatureExtractorState,
    active_note: Option<RecordNote>,
    thresh_hold: f32,
    note_rb: HeapProd<(Note, u128)>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FeatureExtractorState {
    Idle,
    Rise,
    Peak,
    Decay,
}

impl NoteFeatureExtractorImpl {
    pub fn new(thresh_hold: f32, heap_prod: HeapProd<(Note, u128)>) -> Self {
        Self {
            state: FeatureExtractorState::Idle,
            active_note: None,
            thresh_hold,
            note_rb: heap_prod,
        }
    }

    pub fn state(&self) -> FeatureExtractorState {
        self.state
    }

    pub fn active_note(&self) -> Option<&RecordNote> {
        self.active_note.as_ref()
    }

    fn start_note(&mut self, pitch: Pitch, octave: Octave, tonality_offset: i8, dbfs: f32) {
        self.active_note = Some(RecordNote::new(pitch, octave, tonality_offset, dbfs));
        self.state = FeatureExtractorState::Rise;
    }

    fn finalize_note(&mut self, time_stamp: u128) {
        if let Some(active) = self.active_note.take() {
            let _ = self.note_rb.try_push((active.into_note(), time_stamp));
        }
    }

    fn reset(&mut self, time_stamp: u128) {
        self.finalize_note(time_stamp);
        self.state = FeatureExtractorState::Idle;
        self.active_note = None;
    }

    fn processing_single_note(
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
        let frequency = mpm.mpm(&filtered_frame, cfg, instrument);

        let detected = if dbfs >= self.thresh_hold {
            get_note(frequency)
        } else {
            None
        };

        match (self.state, detected) {
            (FeatureExtractorState::Idle, Some((pitch, octave, tonality_offset))) => {
                self.start_note(pitch, octave, tonality_offset, dbfs);
            }
            (FeatureExtractorState::Idle, None) => {}
            (FeatureExtractorState::Rise, Some((pitch, octave, tonality_offset))) => {
                if let Some(ref mut active) = self.active_note {
                    if active.pitch == pitch && active.octave == octave {
                        active.tonality_offset = tonality_offset;
                        if dbfs >= active.peak_dbfs {
                            active.add_peak_dbfs(dbfs);
                        } else {
                            active.record_rise_time();
                            self.state = FeatureExtractorState::Decay;
                        }
                    } else {
                        self.finalize_note(timestamp);
                        self.start_note(pitch, octave, tonality_offset, dbfs);
                    }
                } else {
                    self.start_note(pitch, octave, tonality_offset, dbfs);
                }
            }
            (FeatureExtractorState::Rise, None) => {
                self.reset(timestamp);
            }
            (FeatureExtractorState::Peak, Some((pitch, octave, tonality_offset))) => {
                if let Some(ref mut active) = self.active_note {
                    if active.pitch == pitch && active.octave == octave {
                        active.record_rise_time();
                        active.tonality_offset = tonality_offset;
                        self.state = FeatureExtractorState::Decay;
                    } else {
                        self.finalize_note(timestamp);
                        self.start_note(pitch, octave, tonality_offset, dbfs);
                    }
                } else {
                    self.start_note(pitch, octave, tonality_offset, dbfs);
                }
            }
            (FeatureExtractorState::Peak, None) => {
                self.reset(timestamp);
            }
            (FeatureExtractorState::Decay, Some((pitch, octave, tonality_offset))) => {
                if let Some(ref mut active) = self.active_note {
                    if active.pitch == pitch && active.octave == octave {
                        active.tonality_offset = tonality_offset;
                    } else {
                        self.finalize_note(timestamp);
                        self.start_note(pitch, octave, tonality_offset, dbfs);
                    }
                } else {
                    self.start_note(pitch, octave, tonality_offset, dbfs);
                }
            }
            (FeatureExtractorState::Decay, None) => {
                self.reset(timestamp);
            }
        }
    }
}

impl DspCallBack for NoteFeatureExtractorImpl {
    fn dsp_callback(
        &mut self,
        CallBackParameters {
            buffer,
            cfg,
            filter,
            instrument,
            mpm,
        }: CallBackParameters,
    ) -> () {
        self.processing_single_note(CallBackParameters {
            buffer,
            cfg,
            filter,
            instrument,
            mpm,
        });
    }
}
