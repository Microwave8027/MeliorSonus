#[path = "card_output/card.rs"]
pub mod card;

#[path = "audio_processing/cpal/cpal.rs"]
pub mod audio_engine;

#[path = "audio_processing/dsp.rs"]
pub mod dsp;

#[path = "utils/errors.rs"]
pub mod errors;

#[path = "utils/guard.rs"]
pub mod guard;

#[path = "audio_processing/instruments/instruments.rs"]
pub mod instruments;

#[path = "audio_processing/instruments/notes.rs"]
pub mod notes;

#[path = "audio_processing/processing/functions/mpm.rs"]
pub mod mpm;

#[path = "audio_processing/processing/functions/nsdf.rs"]
pub mod nsdf;

#[path = "audio_processing/processing/functions/high_pass_filter.rs"]
pub mod high_pass_filter;

#[path = "audio_processing/processing/feature_extraction/feature_extractor.rs"]
pub mod feature_extractor;

#[path = "audio_processing/processing/feature_extraction/polyphonic_feature_extractor.rs"]
pub mod polyphonic_feature_extractor;

#[path = "audio_processing/processing/functions/rms_dbfs.rs"]
pub mod rms_dbfs;

#[cfg(test)]
#[path = "tests/dsp_tests/mod.rs"]
pub mod dsp_tests;
