use crate::audio_processing::processing::functions::filters::band_pass_filter::BandPassFilter;
use crate::tests::helpers::make_tone_frame;

#[test]
fn test_band_pass_filter_attenuation() {
    let mut filter = BandPassFilter::new(200.0, 2000.0, 44100);
    let low_rumble = make_tone_frame(30.0, 1.0, 44100);
    let pass_tone = make_tone_frame(1000.0, 1.0, 44100);

    let filtered_rumble = filter.process_frames(&low_rumble);
    let filtered_pass = filter.process_frames(&pass_tone);

    let max_rumble = filtered_rumble
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x.abs()));
    let max_pass = filtered_pass
        .iter()
        .fold(0.0f32, |acc, &x| acc.max(x.abs()));

    // Out-of-band 30 Hz signal must be attenuated significantly compared to in-band 1000 Hz
    assert!(max_rumble < max_pass * 0.2);
}

#[test]
fn test_piano_filter_and_mpm_across_sample_rates() {
    use crate::audio_processing::instruments::instrument::Instrument;
    use crate::audio_processing::processing::functions::pitch::mpm::MPM;
    use crate::constants::FRAME_SIZE;
    use cpal::StreamConfig;

    for &sr in &[16000, 44100, 48000] {
        let inst = Instrument::Piano;
        let range = inst.filter_range();
        let mut filter = BandPassFilter::new(range.hpf_cutoff_hz, range.harmonic_ceiling_hz, sr);
        let mut mpm = MPM::new(FRAME_SIZE / 2);
        let cfg = StreamConfig {
            channels: 1,
            sample_rate: sr,
            buffer_size: cpal::BufferSize::Default,
        };

        let raw = make_tone_frame(440.0, 0.5, sr);
        let filtered = filter.process_frames(&raw);
        let (freq, clarity) = mpm.mpm(&filtered, &cfg, &inst);
        println!("SR: {} -> Raw max: {}, Filtered max: {}, MPM: ({} Hz, clarity {})",
            sr,
            raw.iter().fold(0.0f32, |a, &x| a.max(x.abs())),
            filtered.iter().fold(0.0f32, |a, &x| a.max(x.abs())),
            freq,
            clarity
        );
    }
}

