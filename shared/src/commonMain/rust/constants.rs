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
pub const BYTEDANCE_CRNN_SAMPLE_RATE: u32 = 16000; // Native sample rate for ByteDance CRNN
pub const BYTEDANCE_CRNN_HOP_SIZE: usize = 320; // 320 samples @ 16kHz = 20ms frame resolution
pub const BYTEDANCE_CRNN_FFT_SIZE: usize = 2048; // 2048 samples @ 16kHz = 128ms STFT window
pub const BYTEDANCE_CRNN_MEL_BINS: usize = 229; // 229 Slaney triangular Mel filterbanks
pub const BYTEDANCE_CRNN_CONTEXT_FRAMES: usize = 32; // 32 frames @ 20ms = 640ms temporal context
pub const BYTEDANCE_CRNN_CONTEXT_LEN: usize = BYTEDANCE_CRNN_CONTEXT_FRAMES * BYTEDANCE_CRNN_MEL_BINS; // 7328 floats


// Global Lazylocks/static values
pub static GLOBAL_AUDIO_METRICS: LazyLock<LiveAudioMetrics> =
    LazyLock::new(|| LiveAudioMetrics::new());

pub struct MxlPaths {
    pub zipped_mxl: String,
    pub parsed_mxl: String,
}
