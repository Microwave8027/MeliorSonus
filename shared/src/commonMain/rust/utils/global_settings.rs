use crate::audio_processing::{CrnnType, HardwareDelegate, HybridPitchDetectorMode};

/// When editing this, the audio engine MUST BE PAUSED
pub struct GlobalSettings {
    pub device: HardwareDelegate,
    pub show_metrics: bool,
    pub note_recognition_mode: HybridPitchDetectorMode,
    pub crnn_type: CrnnType,
}

impl Default for GlobalSettings {
    fn default() -> Self {
        Self {
            device: HardwareDelegate::Cpu,
            show_metrics: false,
            note_recognition_mode: HybridPitchDetectorMode::Mpm,
            crnn_type: CrnnType::ByteDance,
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
            crnn_type: CrnnType::ByteDance,
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

    #[inline]
    /// When editing this, the audio engine MUST BE PAUSED
    pub fn change_crnn_type(&mut self, crnn_type: CrnnType) {
        self.crnn_type = crnn_type;
    }
}
