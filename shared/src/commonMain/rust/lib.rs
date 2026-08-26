uniffi::setup_scaffolding!();

pub mod audio_processing;
pub mod card_output;
pub mod constants;
pub mod utils;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::audio_processing::cpal::engine::AudioEngine;
    pub use crate::audio_processing::dsp::{CallBackParameters, Dsp, DspCallBack};
    pub use crate::audio_processing::instruments::instrument::{FilterRange, Instrument, MpmConfig};
    pub use crate::audio_processing::instruments::notes::{
        EndNote, Notes, Octave, Pitch, RecordNote, StartNote,
    };
    pub use crate::audio_processing::processing::feature_extraction::dsp_feature_extractor::DspFeatureExtractor;
    pub use crate::audio_processing::processing::feature_extraction::monophonic_feature_extractor::{
        FeatureExtractorState, NoteFeatureExtractorImpl,
    };
    pub use crate::audio_processing::processing::feature_extraction::polyphonic_feature_extractor::{
        PolyphonicFeatureExtractorImpl, PolyphonicNoteState, PolyphonyMode,
    };
    pub use crate::audio_processing::processing::functions::harmonic_sieve_mask::HarmonicSieveMasker;
    pub use crate::audio_processing::processing::functions::high_pass_filter::BandPassFilter;
    pub use crate::audio_processing::processing::functions::mpm::MPM;
    pub use crate::audio_processing::processing::functions::nsdf::NsdfEvaluator;
    pub use crate::audio_processing::processing::functions::rms_dbfs::loudness;
    pub use crate::card_output::card::{Card, CardType, test};
    pub use crate::constants::*;
    pub use crate::utils::error_callback::ErrorCallback;
    pub use crate::utils::errors::RustError;
    pub use crate::utils::feature_extractor_state::{track_note_state, NoteEnvelopeState};
    pub use crate::utils::guard::DropGuard;
    pub use cpal::{Stream, StreamConfig};
    pub use ringbuf::{traits::*, HeapCons, HeapProd, HeapRb};
    pub use std::error::Error;
    pub use std::sync::atomic::{AtomicBool, Ordering};
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
}
