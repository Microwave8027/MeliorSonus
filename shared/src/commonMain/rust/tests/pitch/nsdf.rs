use crate::audio_processing::processing::functions::pitch::nsdf::NsdfEvaluator;
use crate::constants::FRAME_SIZE;
use crate::tests::helpers::make_tone_frame;

#[test]
fn test_nsdf_monophonic_vs_chord() {
    let mut nsdf = NsdfEvaluator::new();

    // 1. Pure monophonic sine wave (A4 = 440 Hz)
    let mono_frame = make_tone_frame(440.0, 0.8, 44100);
    let (mono_clarity, mono_peak_ratio) = nsdf.evaluate_frame(&mono_frame);

    assert!(
        mono_clarity > 0.85,
        "Monophonic sine should have high NSDF clarity, got {}",
        mono_clarity
    );
    assert!(
        mono_peak_ratio < 0.6,
        "Monophonic sine should have low secondary peak ratio, got {}",
        mono_peak_ratio
    );

    // 2. Complex dissonant dyad / chord (440 Hz + 466.16 Hz + 554.37 Hz)
    let mut chord_frame = [0.0f32; FRAME_SIZE];
    let f1 = make_tone_frame(440.0, 0.4, 44100);
    let f2 = make_tone_frame(466.16, 0.4, 44100);
    let f3 = make_tone_frame(554.37, 0.4, 44100);
    for i in 0..FRAME_SIZE {
        chord_frame[i] = f1[i] + f2[i] + f3[i];
    }

    let (chord_clarity, chord_peak_ratio) = nsdf.evaluate_frame(&chord_frame);
    // Polyphonic / complex signals exhibit either reduced clarity or elevated secondary NSDF peak ratios
    assert!(
        chord_clarity < mono_clarity || chord_peak_ratio > mono_peak_ratio,
        "Chord should show polyphonic characteristics vs pure mono"
    );
}
