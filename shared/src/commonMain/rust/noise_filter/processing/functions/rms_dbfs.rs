pub fn loudness(raw_bytes: [f32; 1024]) -> f32 {
    let mut rms = 0.0f32;
    for byte in raw_bytes.iter() {
        rms += byte.powi(2);
    }
    rms /= raw_bytes.len() as f32;
    let dbfs = 20.0 * (rms.powf(0.5) + 1e-9).log10();

    dbfs
}
