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

#[test]
fn test_hybrid_feature_extractor_bytedance_crnn_pipeline() {
    let (prod, _cons) = RingBuffer::<Notes>::new(64);
    let settings = Arc::new(Mutex::new(GlobalSettings {
        device: HardwareDelegate::Cpu,
        show_metrics: true,
        note_recognition_mode: crate::audio_processing::HybridPitchDetectorMode::Crnn,
        crnn_type: crate::audio_processing::CrnnType::ByteDance,
    }));

    let mut extractor = HybridFeatureExtractor::new(
        prod,
        48000,
        -45.0,
        PitchDetectorMode::Crnn,
        settings,
    )
    .expect("valid extractor with ByteDance CRNN");

    assert_eq!(extractor.crnn_type, crate::audio_processing::CrnnType::ByteDance);
    assert!(extractor.resampler_16k.is_some());
    assert!(extractor.bytedance_crnn.is_some());

    let cfg = StreamConfig {
        channels: 1,
        sample_rate: 48000,
        buffer_size: cpal::BufferSize::Default,
    };
    let mut filter = BandPassFilter::new(30.0, 10000.0, 48000);
    let instrument = Instrument::Piano;
    let mut mpm = MPM::new(FRAME_SIZE / 2);

    let a4_frame = make_tone_frame(440.0, 0.7, 48000);

    // Process 20 frames of audio at 48 kHz (512-sample hops = ~10.67ms each)
    // Resamples 48k -> 16k (3:1 integer decimation), pushes ~171 samples per hop into 16k FIFO.
    // Every ~2 hops (342 samples >= 320), Slaney Mel STFT and CRNN inference are triggered.
    for _ in 0..20 {
        extractor.dsp_callback(CallBackParameters {
            buffer: &a4_frame,
            cfg: &cfg,
            filter: &mut filter,
            instrument: &instrument,
            mpm: &mut mpm,
        });
    }

    // Extractor must process smoothly without panic
    assert!(extractor.processed_samples > 0);
}

#[test]
fn test_hybrid_feature_extractor_mode_and_crnn_switch() {
    let (prod, _cons) = RingBuffer::<Notes>::new(64);
    let settings = Arc::new(Mutex::new(GlobalSettings {
        device: HardwareDelegate::Cpu,
        show_metrics: false,
        note_recognition_mode: crate::audio_processing::HybridPitchDetectorMode::Crnn,
        crnn_type: crate::audio_processing::CrnnType::ByteDance,
    }));

    let mut extractor = HybridFeatureExtractor::new(
        prod,
        44100,
        -50.0,
        PitchDetectorMode::Crnn,
        settings,
    )
    .expect("valid extractor");

    assert_eq!(extractor.crnn_type, crate::audio_processing::CrnnType::ByteDance);
    assert!(extractor.resampler_16k.is_some());
    assert!(extractor.bytedance_crnn.is_some());
    assert!(extractor.resampler.is_none());
    assert!(extractor.transcriber.is_none());

    // Switch to BasicPitch
    extractor.change_crnn_type(crate::audio_processing::CrnnType::BasicPitch);
    assert_eq!(extractor.crnn_type, crate::audio_processing::CrnnType::BasicPitch);
    assert!(extractor.resampler_16k.is_none());
    assert!(extractor.bytedance_crnn.is_none());
    assert!(extractor.resampler.is_some());
    assert!(extractor.transcriber.is_some());

    // Switch to Basic mode (pure MPM)
    extractor.change_mode(PitchDetectorMode::Basic);
    assert!(extractor.resampler.is_none());
    assert!(extractor.transcriber.is_none());
    assert!(extractor.resampler_16k.is_none());
    assert!(extractor.bytedance_crnn.is_none());

    // Switch back to ByteDance CRNN
    extractor.change_crnn_type(crate::audio_processing::CrnnType::ByteDance);
    extractor.change_mode(PitchDetectorMode::Crnn);
    assert_eq!(extractor.crnn_type, crate::audio_processing::CrnnType::ByteDance);
    assert!(extractor.resampler_16k.is_some());
    assert!(extractor.bytedance_crnn.is_some());
}

#[test]
fn test_bytedance_static_model_path_configuration() {
    use crate::audio_processing::dsp::{
        set_bytedance_model_paths, BYTEDANCE_ONNX_MODEL_PATH, BYTEDANCE_TFLITE_MODEL_PATH,
    };

    set_bytedance_model_paths(
        "models/bytedance_crnn.tflite".to_string(),
        "models/bytedance_crnn.onnx".to_string(),
    );

    assert_eq!(
        BYTEDANCE_TFLITE_MODEL_PATH.get().map(|s| s.as_str()),
        Some("models/bytedance_crnn.tflite")
    );
    assert_eq!(
        BYTEDANCE_ONNX_MODEL_PATH.get().map(|s| s.as_str()),
        Some("models/bytedance_crnn.onnx")
    );
}


