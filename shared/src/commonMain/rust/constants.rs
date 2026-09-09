use crate::utils::audio_metrics::LiveAudioMetrics;
use std::sync::LazyLock;

// [polyphonic audio extractor]
pub const MAX_POLYPHONY: usize = 16;
pub const PITCH_BINS: usize = 88; // Piano keys A0 (MIDI 21) to C8 (MIDI 108)
pub const MIDI_OFFSET: usize = 21;
pub const NUM_MIDI_NOTES: usize = 128;
pub const HANGOVER_FRAMES_DEFAULT: u8 = 5;

// [CPAL Audio Engine]
pub const FRAME_SIZE: usize = 1024;
pub const RINGBUF_CAPACITY: usize = 16384;
pub const NOTE_RINGBUF_CAPACITY: usize = 1024;
pub const PREFERRED_RATES: [u32; 2] = [44100, 48000];
pub const HOP_SIZE: usize = FRAME_SIZE / 2;
pub const CRNN_INPUT_SIZE: u32 = 22050; // The input sampling rate of the basic pitch crnn from spotify
pub const BASIC_PITCH_HOP_SIZE: usize = 256; // 256 samples @ 22.05kHz matching 512-sample hop @ 44.1kHz

// Global Lazylocks/static values
pub static GLOBAL_AUDIO_METRICS: LazyLock<LiveAudioMetrics> =
    LazyLock::new(|| LiveAudioMetrics::new());

pub struct MxlPaths {
    pub zipped_mxl: String,
    pub parsed_mxl: String,
}
