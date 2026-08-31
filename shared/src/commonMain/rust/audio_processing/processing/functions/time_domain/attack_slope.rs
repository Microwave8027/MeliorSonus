/// Calculates the transient attack slope in dB per second.
/// 
/// A high slope (> 1000 dB/s, or > 1.0 dB/ms) corresponds to a sharp percussive onset.
pub fn calculate_attack_slope(peak_dbfs: f32, onset_dbfs: f32, rise_duration_sec: f32) -> f32 {
    let duration = rise_duration_sec.max(0.001);
    (peak_dbfs - onset_dbfs) / duration
}
