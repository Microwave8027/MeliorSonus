uniffi::setup_scaffolding!();

pub mod audio_analysis;
pub mod audio_processing;
pub mod card_output;
pub mod constants;
pub mod utils;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::audio_analysis::*;
    pub use crate::audio_processing::cpal::engine::AudioEngine;
    pub use crate::audio_processing::dsp::*;
    pub use crate::audio_processing::instruments::instrument::{
        FilterRange, Instrument, InstrumentAcousticProfile, MpmConfig,
    };
    pub use crate::audio_processing::instruments::notes::*;
    pub use crate::audio_processing::neural::*;
    pub use crate::audio_processing::processing::feature_extraction::hybrid_extractor::HybridFeatureExtractor;
    pub use crate::audio_processing::processing::functions::articulation::*;
    pub use crate::audio_processing::processing::functions::filters::*;
    pub use crate::audio_processing::processing::functions::pitch::*;
    pub use crate::audio_processing::processing::functions::resampling::*;
    pub use crate::audio_processing::processing::functions::spectral::*;
    pub use crate::audio_processing::processing::functions::time_domain::*;
    pub use crate::audio_processing::{HardwareDelegate, TractBasicPitchModel};
    pub use crate::card_output::card::{Card, CardType, test};
    pub use crate::utils::error_callback::ErrorCallback;
    pub use crate::utils::errors::RustError;
    pub use crate::utils::guard::DropGuard;
    pub use cpal::{Stream, StreamConfig};
    pub use rtrb::{Consumer, Producer, RingBuffer};
    pub use std::error::Error;
    pub use std::sync::atomic::{AtomicBool, Ordering};
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
}

#[derive(uniffi::Object)]
pub struct AudioAnalyzer;

impl AudioAnalyzer {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self
    }
}
