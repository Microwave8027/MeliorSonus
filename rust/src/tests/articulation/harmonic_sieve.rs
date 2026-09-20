use crate::audio_processing::processing::functions::articulation::harmonic_sieve::HarmonicSieveMasker;

#[test]
fn test_harmonic_sieve_masker() {
    let mut sieve = HarmonicSieveMasker::new(0.1, 4);

    let mut raw_probs = [0.0f32; 88];
    // Set fundamental at bin 48 (A4 = 440 Hz)
    raw_probs[48] = 0.9;
    // Overtones at bin 60 (A5 = 880 Hz) and bin 67 (E6 ~1320 Hz)
    raw_probs[60] = 0.4;
    raw_probs[67] = 0.3;
    // Independent chord note at bin 52 (C#5)
    raw_probs[52] = 0.85;

    let mut output_probs = [0.0f32; 88];
    sieve.apply_sieve(&raw_probs, 44100.0, &mut output_probs);

    assert_eq!(output_probs[48], 0.9);
    assert_eq!(output_probs[52], 0.85);
    assert!(output_probs[60] < 0.4);
}
