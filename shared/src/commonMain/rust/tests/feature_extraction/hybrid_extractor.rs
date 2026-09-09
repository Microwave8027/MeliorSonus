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
use crate::tests::helpers::make_tone_frame;
use crate::utils::global_settings::GlobalSettings;
use cpal::StreamConfig;
use rtrb::RingBuffer;
use std::sync::{Arc, Mutex};

#[test]
fn test_hybrid_feature_extractor_pipeline_live() {
    let (prod, mut cons) = RingBuffer::<Notes>::new(32);
    let mut hybrid = HybridFeatureExtractor::new(
        prod,
        44100,
        -45.0,
        PitchDetectorMode::Basic,
        HardwareDelegate::Cpu,
        Arc::new(Mutex::new(GlobalSettings::default())),
    )
    .expect("valid extractor");

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

    // 1. Process 10 frames of A4 tone
    for _ in 0..10 {
        hybrid.dsp_callback(CallBackParameters {
            buffer: &a4_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    let event1 = cons.pop().expect("Should produce StartNote");
    assert_eq!(event1.pitch(), Pitch::A);
    assert_eq!(event1.octave(), Octave::O4);
    assert!(event1.is_start());

    // 2. Process silence frames to finalize note (clear 1024-sample sliding window and release hangover)
    for _ in 0..4 {
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
    assert!(event2.is_end());
}

#[test]
fn test_hybrid_feature_extractor_full_lifecycle() {
    let (prod, mut cons) = RingBuffer::<Notes>::new(64);
    let mut extractor = HybridFeatureExtractor::new(
        prod,
        44100,
        -45.0,
        PitchDetectorMode::Basic,
        HardwareDelegate::Cpu,
        Arc::new(Mutex::new(GlobalSettings::default())),
    )
    .expect("valid extractor");

    extractor.reset_stream_time(1000);

    let cfg = StreamConfig {
        channels: 1,
        sample_rate: 44100,
        buffer_size: cpal::BufferSize::Default,
    };
    let mut filter = BandPassFilter::new(30.0, 10000.0, 44100);
    let instrument = Instrument::Piano;
    let mut mpm = MPM::new(FRAME_SIZE / 2);

    let a4_frame = make_tone_frame(440.0, 0.7, 44100);
    let silent_frame = [0.0f32; FRAME_SIZE];

    // 1. Process 10 frames of A4
    for _ in 0..10 {
        extractor.dsp_callback(CallBackParameters {
            buffer: &a4_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    let start_note = cons.pop().expect("Should have emitted StartNote");
    assert_eq!(start_note.pitch(), Pitch::A);
    assert_eq!(start_note.octave(), Octave::O4);

    // 2. Silence triggers finalize_all / note end (clear 1024-sample sliding window and release hangover)
    for _ in 0..4 {
        extractor.dsp_callback(CallBackParameters {
            buffer: &silent_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    let end_note = cons.pop().expect("Should have emitted EndNote");
    assert_eq!(end_note.pitch(), Pitch::A);
    assert_eq!(end_note.octave(), Octave::O4);
    if let Notes::End(e) = end_note {
        assert!(e.note_duration > 0.05);
    }
}
