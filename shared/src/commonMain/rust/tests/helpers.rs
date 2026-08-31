use crate::constants::FRAME_SIZE;
use crate::utils::error_callback::ErrorCallback;
use crate::utils::errors::RustError;

pub struct TestErrorCallback;
impl ErrorCallback for TestErrorCallback {
    fn on_error(&self, _msg: RustError) {}
    fn on_complete(&self) {}
}

pub fn make_tone_frame(freq_hz: f32, amplitude: f32, sample_rate: u32) -> [f32; FRAME_SIZE] {
    let mut frame = [0.0f32; FRAME_SIZE];
    for (i, x) in frame.iter_mut().enumerate() {
        *x = amplitude
            * (2.0 * std::f32::consts::PI * freq_hz * (i as f32) / (sample_rate as f32)).sin();
    }
    frame
}
