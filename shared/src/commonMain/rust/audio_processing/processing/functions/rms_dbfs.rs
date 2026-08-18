/*
* Output range of dbfs with rms:
* 0.0 means that the frequency is constantly at a maximum amplitude
* -3.0 means that the the sine wave is peaking
* -12 to -6 means that the it is a loud signal
* -24 to -18 represents a normal, average signal
* -60 - -40 represents soft and queit noises like instrumental decay
* -96 represents the silence limit for 16 bit audio
* -144 is the noise floor for 24 bit audio
* -180 this is pure digital silence
* below -40 is practically silence, but additional calibration may be added where the threshold is represented as a const or static
*/
pub fn loudness(raw_bytes: &[f32]) -> f32 {
    let mut rms = 0.0f32;
    for byte in raw_bytes.iter() {
        rms += byte.powi(2);
    }
    rms /= raw_bytes.len() as f32;
    let dbfs = 20.0 * (rms.powf(0.5) + 1e-9).log10();

    dbfs
}
