// [polyphonic audio extractor]
pub const MAX_POLYPHONY: usize = 16;
pub const PITCH_BINS: usize = 88; // Piano keys A0 (MIDI 21) to C8 (MIDI 108)
pub const HANGOVER_FRAMES_DEFAULT: u8 = 5;

// [CPAL Audio Engine]
pub const FRAME_SIZE: usize = 1024;
pub const RINGBUF_CAPACITY: usize = 16384;
pub const NOTE_RINGBUF_CAPACITY: usize = 1024;
pub const PREFERRED_RATES: [u32; 2] = [44100, 48000];
pub const HOP_SIZE: usize = FRAME_SIZE / 2;
