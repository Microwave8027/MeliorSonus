use crate::audio_processing::cpal::engine::AudioEngine;
use crate::audio_processing::dsp::{CallBackParameters, DspCallBack};
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::litert_model::HardwareDelegate;
use crate::audio_processing::processing::feature_extraction::current_feature_extractor::hybrid_extractor::{
    HybridFeatureExtractor, PitchDetectorMode,
};
use crate::audio_processing::processing::functions::filters::band_pass_filter::BandPassFilter;
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::constants::FRAME_SIZE;
use crate::tests::helpers::{TestErrorCallback, make_tone_frame};
use cpal::StreamConfig;
use rtrb::RingBuffer;
use std::sync::Arc;

#[test]
fn test_hybrid_feature_extractor_audio_engine_direct_integration() {
    let (prod, mut cons) = RingBuffer::<Notes>::new(32);
    let mut hybrid = HybridFeatureExtractor::new(
        prod,
        44100,
        -45.0,
        PitchDetectorMode::Basic,
        HardwareDelegate::Cpu,
    )
    .expect("valid extractor");

    hybrid.reset_stream_time(0);

    let cfg = StreamConfig {
        channels: 1,
        sample_rate: 44100,
        buffer_size: cpal::BufferSize::Default,
    };
    let mut filter = BandPassFilter::new(30.0, 10000.0, 44100);
    let instrument = Instrument::Piano;
    let mut mpm = MPM::new(FRAME_SIZE / 2);

    let a4_frame = make_tone_frame(440.0, 0.8, 44100);
    let silent_frame = [0.0f32; FRAME_SIZE];

    // Process 10 frames of A4 tone (10 * 512 / 44100 = ~116ms > 50ms min duration)
    for _ in 0..10 {
        hybrid.dsp_callback(CallBackParameters {
            buffer: &a4_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    // Verify that StartNote was pushed into ringbuffer
    let event1 = cons.pop().expect("Should produce StartNote");
    assert_eq!(event1.pitch(), Pitch::A);
    assert_eq!(event1.octave(), Octave::O4);
    if let Notes::Start(s) = event1 {
        assert_eq!(s.dynamic, DynamicLevel::Fortississimo);
        assert!(s.velocity > 50);
    } else {
        panic!("Expected Notes::Start");
    }

    // Send silence frames to trigger release (2 hops to clear 1024-sample sliding window)
    for _ in 0..2 {
        hybrid.dsp_callback(CallBackParameters {
            buffer: &silent_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    let event2 = cons.pop().expect("Should produce EndNote on silence");
    assert_eq!(event2.pitch(), Pitch::A);
    assert_eq!(event2.octave(), Octave::O4);

    // Verify AudioEngine struct compiles with rtrb Producer
    let (prod2, _cons2) = RingBuffer::<Notes>::new(32);
    let engine = AudioEngine::new(
        Instrument::Piano,
        Arc::new(TestErrorCallback),
        prod2,
        -45.0,
        PitchDetectorMode::Basic,
        HardwareDelegate::Cpu,
    );
    assert!(!engine.is_playing.load(std::sync::atomic::Ordering::Relaxed));
}
