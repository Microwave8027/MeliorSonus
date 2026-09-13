use crate::audio_processing::{HardwareDelegate, HybridPitchDetectorMode};

/// When editing this, the audio engine MUST BE PAUSED
pub struct GlobalSettings {
    pub device: HardwareDelegate,
    pub show_metrics: bool,
    pub note_recognition_mode: HybridPitchDetectorMode,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            device: HardwareDelegate::Cpu,
            show_metrics: false,
            note_recognition_mode: HybridPitchDetectorMode::Mpm,
        }
    }
}

impl GlobalSettings {
    pub fn new(
        device: HardwareDelegate,
        show_metrics: bool,
        note_recognition_mode: HybridPitchDetectorMode,
    ) -> Self {
        Self {
            device,
            show_metrics,
            note_recognition_mode,
        }
    }

    /// When editing this, the audio engine MUST BE PAUSED
    pub fn change_device(&mut self, device: HardwareDelegate) {
        self.device = device;
    }

    /// When editing this, the audio engine MUST BE PAUSED
    pub fn change_metrics_mode(&mut self, show_metrics: bool) {
        self.show_metrics = show_metrics;
    }

    #[inline]
    /// When editing this, the audio engine MUST BE PAUSED
    pub fn change_note_recognition_mode(&mut self, note_recognition_mode: HybridPitchDetectorMode) {
        self.note_recognition_mode = note_recognition_mode;
    }
}
