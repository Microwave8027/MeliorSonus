uniffi::setup_scaffolding!();

mod paths;
pub use paths::*;

pub mod prelude {
    pub use crate::audio_engine::AudioEngine;
    pub use crate::card::{Card, CardType};
    pub use crate::dsp::{CallBackParameters, DspCallBack};
    pub use crate::errors::*;
    pub use crate::guard::DropGuard;
    pub use crate::instruments::{FilterRange, Instrument, MpmConfig};
    pub use crate::monophonic_feature_extractor::NoteFeatureExtractorImpl;
    pub use crate::polyphonic_feature_extractor::{
        PolyphonicFeatureExtractorImpl, PolyphonicNoteState, PolyphonyMode,
    };
    pub use crate::dsp_feature_extractor::{dsp_feature_extractor, DspFeatureExtractor};
    pub use crate::processor::{track_note_state, NoteEnvelopeState};
    pub use cpal::{Stream, StreamConfig};
    pub use std::error::Error;
    pub use std::sync::atomic::{AtomicBool, Ordering};
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
}

// constants
pub mod constants {
    // [polyphonic audio extractor]
    pub const MAX_POLYPHONY: usize = 16;
    pub const PITCH_BINS: usize = 88; // Piano keys A0 (MIDI 21) to C8 (MIDI 108)
    pub const HANGOVER_FRAMES_DEFAULT: u8 = 5;

    // [CPAL Audio Engine]
    pub const FRAME_SIZE: usize = 1024;
    pub const RINGBUF_CAPACITY: usize = 16384;
    pub const PREFERRED_RATES: [u32; 2] = [44100, 48000];
    pub const HOP_SIZE: usize = FRAME_SIZE / 2;
}

// Logger likely will not be used as i dont want to debug a logger and errors can generally be handled
/*
#[uniffi::export]
pub fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
            .with_tag("RustCore"),
    );
}
*/
