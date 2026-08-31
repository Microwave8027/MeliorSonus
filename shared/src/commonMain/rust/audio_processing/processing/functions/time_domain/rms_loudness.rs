pub fn loudness(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -180.0;
    }
    let mut rms: f32 = samples.iter().map(|&x| x * x).sum();
    rms /= samples.len() as f32;
    20.0 * (rms.powf(0.5) + 1e-9).log10()
}
