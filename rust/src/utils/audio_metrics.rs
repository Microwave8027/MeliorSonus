use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[derive(Debug)]
pub struct LiveAudioMetrics {
    pub rms_dbfs_bits: AtomicU32,
    pub mpm_freq_bits: AtomicU32,
    pub clarity_bits: AtomicU32,
    pub processed_frames: AtomicU64,
}

impl Default for LiveAudioMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveAudioMetrics {
    pub fn new() -> Self {
        Self {
            rms_dbfs_bits: AtomicU32::new((-120.0f32).to_bits()),
            mpm_freq_bits: AtomicU32::new(0.0f32.to_bits()),
            clarity_bits: AtomicU32::new(0.0f32.to_bits()),
            processed_frames: AtomicU64::new(0),
        }
    }

    pub fn rms_dbfs(&self) -> f32 {
        f32::from_bits(self.rms_dbfs_bits.load(Ordering::Relaxed))
    }

    pub fn mpm_freq(&self) -> f32 {
        f32::from_bits(self.mpm_freq_bits.load(Ordering::Relaxed))
    }

    pub fn clarity(&self) -> f32 {
        f32::from_bits(self.clarity_bits.load(Ordering::Relaxed))
    }

    pub fn snapshot(&self) -> (f32, f32, f32, u64) {
        (
            self.rms_dbfs(),
            self.mpm_freq(),
            self.clarity(),
            self.processed_frames.load(Ordering::Relaxed),
        )
    }
}
