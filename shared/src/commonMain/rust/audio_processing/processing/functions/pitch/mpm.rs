use crate::audio_processing::instruments::instrument::Instrument;
use crate::constants::*;
use cpal::StreamConfig;
use pitch_detection::detector::PitchDetector;
use pitch_detection::detector::mcleod::McLeodDetector;

pub struct MPM {
    pitch_detector: McLeodDetector<f32>,
}

// SAFETY: Moved only to the single audio thread
unsafe impl Send for MPM {}

impl MPM {
    pub fn new(padding: usize) -> Self {
        MPM {
            pitch_detector: McLeodDetector::new(FRAME_SIZE, padding),
        }
    }

    pub fn mpm(
        &mut self,
        frame: &[f32; FRAME_SIZE],
        cfg: &StreamConfig,
        instrument: &Instrument,
    ) -> (f32, f32) {
        let sample_rate = cfg.sample_rate as usize;
        let config = instrument.mpm_config();
        let range = instrument.filter_range();

        let pitch = self.pitch_detector.get_pitch(
            &frame[..],
            sample_rate,
            config.power_threshold,
            config.clarity_threshold,
        );
        match pitch {
            Some(p) if p.frequency >= range.min_f0_hz && p.frequency <= range.max_f0_hz => {
                (p.frequency as f32, p.clarity as f32)
            }
            _ => (0.0f32, 0.0f32),
        }
    }
}
