use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::constants::FRAME_SIZE;
use crate::tests::helpers::make_tone_frame;
use cpal::StreamConfig;

#[test]
fn test_mpm_pitch_tracking_monophonic() {
    let mut mpm = MPM::new(FRAME_SIZE / 2);
    let cfg = StreamConfig {
        channels: 1,
        sample_rate: 44100,
        buffer_size: cpal::BufferSize::Default,
    };
    let instrument = Instrument::Piano;

    let a4_frame = make_tone_frame(440.0, 0.8, 44100);
    let (freq, clarity) = mpm.mpm(&a4_frame, &cfg, &instrument);

    assert!((freq - 440.0).abs() < 1.5, "Expected ~440 Hz, got {}", freq);
    assert!(clarity > 0.9, "Expected clarity > 0.9, got {}", clarity);
}

#[test]
fn test_mpm_various_sample_rates() {
    for &sr in &[16000, 44100, 48000, 96000] {
        let mut mpm = MPM::new(FRAME_SIZE / 2);
        let cfg = StreamConfig {
            channels: 1,
            sample_rate: sr,
            buffer_size: cpal::BufferSize::Default,
        };
        let instrument = Instrument::Piano;

        for &target_freq in &[220.0, 440.0, 880.0] {
            let tone = make_tone_frame(target_freq, 0.8, sr);
            let (freq, clarity) = mpm.mpm(&tone, &cfg, &instrument);
            println!("SR: {}, Target: {} Hz -> Detected: {} Hz, Clarity: {}", sr, target_freq, freq, clarity);
        }
    }
}

