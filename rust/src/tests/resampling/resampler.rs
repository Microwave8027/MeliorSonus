use crate::audio_processing::processing::functions::resampling::AudioResampler;

#[test]
fn test_resampler_44100_to_22050() {
    let mut resampler = AudioResampler::new(44100, 22050, 512);
    let input_44k = [0.5f32; 512];

    // First chunk warms up the FFT resampler delay line
    let _ = resampler.process_chunk(&input_44k);
    let out = resampler
        .process_chunk(&input_44k)
        .expect("Resampling should succeed");
    // Decimates 2:1 -> 512 / 2 = 256 samples
    assert_eq!(out.len(), 256);
}
