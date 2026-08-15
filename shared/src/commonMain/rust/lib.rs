uniffi::setup_scaffolding!();

mod paths;
pub use paths::*;

pub mod prelude {
    pub use crate::audio_engine::AudioEngine;
    pub use crate::card::{Card, CardType};
    pub use crate::dsp::DspCallBack;
    pub use crate::errors::*;
    pub use crate::guard::DropGuard;
    pub use cpal::{Stream, StreamConfig};
    pub use std::error::Error;
    pub use std::sync::atomic::{AtomicBool, Ordering};
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
    pub const FRAME_SIZE: usize = 1024;
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
