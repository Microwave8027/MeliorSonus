pub const ONSET_WINDOW_SIZE: usize = 7;
pub const DEFAULT_HFC_THRESHOLD: f32 = 2.2;
pub const DEFAULT_SILENCE_THRESHOLD_DBFS: f32 = -45.0;
pub const DEFAULT_MIN_IOI_MS: u128 = 60; // 60ms debounce prevents mechanical strike double-triggers

/// Computes the High-Frequency Content (HFC) of an FFT magnitude spectrum.
///
/// Formula: HFC = sum_{k=0}^{N/2} (k + 1) * |X[k]|^2
/// Higher spectral bins are linearly weighted to accentuate transient burst energy.
#[inline(always)]
pub fn compute_hfc(magnitudes: &[f32]) -> f32 {
    let mut hfc = 0.0f32;
    for (k, &mag) in magnitudes.iter().enumerate() {
        let weight = (k + 1) as f32;
        hfc += weight * mag * mag;
    }
    hfc
}

/// Standalone real-time zero-allocation HFC Onset and Transient Detector.
///
/// Features:
/// Sliding 7-frame sliding window with local peak-picking around window midpoint.
/// Adaptive statistical thresholding: Candidate >= Median(Window) + C * StdDev(Window).
/// RMS silence gating to suppress noise floor jitter.
/// Minimum Inter-Onset Interval (MinIOI) debounce to eliminate flutter and multiple-triggers.
#[derive(Debug, Clone)]
pub struct HfcOnsetDetector {
    threshold: f32,
    silence_threshold_dbfs: f32,
    min_ioi_ms: u128,

    odf_window: [f32; ONSET_WINDOW_SIZE],
    timestamp_window: [u128; ONSET_WINDOW_SIZE],
    loudness_window: [f32; ONSET_WINDOW_SIZE],
    window_count: usize,
    last_onset_timestamp_ms: u128,
}

impl HfcOnsetDetector {
    pub fn new(threshold: f32, silence_threshold_dbfs: f32, min_ioi_ms: u128) -> Self {
        Self {
            threshold,
            silence_threshold_dbfs,
            min_ioi_ms,
            odf_window: [0.0; ONSET_WINDOW_SIZE],
            timestamp_window: [0; ONSET_WINDOW_SIZE],
            loudness_window: [-120.0; ONSET_WINDOW_SIZE],
            window_count: 0,
            last_onset_timestamp_ms: 0,
        }
    }

    pub fn default_detector() -> Self {
        Self::new(
            DEFAULT_HFC_THRESHOLD,
            DEFAULT_SILENCE_THRESHOLD_DBFS,
            DEFAULT_MIN_IOI_MS,
        )
    }

    pub fn history_len(&self) -> usize {
        self.window_count
    }

    /// Resets detector history and buffers.
    pub fn reset(&mut self) {
        self.odf_window.fill(0.0);
        self.timestamp_window.fill(0);
        self.loudness_window.fill(-120.0);
        self.window_count = 0;
        self.last_onset_timestamp_ms = 0;
    }

    /// Feeds a new frame's magnitude spectrum, loudness in dBFS, and timestamp in ms.
    ///
    /// Returns `Some(timestamp_ms)` when a transient onset attack is confirmed, or `None`.\
    pub fn process_frame(
        &mut self,
        magnitudes: &[f32],
        loudness_dbfs: f32,
        timestamp_ms: u128,
    ) -> Option<u128> {
        let odf_val = compute_hfc(magnitudes);
        self.process_odf_value(odf_val, loudness_dbfs, timestamp_ms)
    }

    /// Feeds a pre-computed ODF value with loudness in dBFS and timestamp in ms.
    pub fn process_odf_value(
        &mut self,
        odf_val: f32,
        loudness_dbfs: f32,
        timestamp_ms: u128,
    ) -> Option<u128> {
        // Shift window left by 1
        for i in 0..(ONSET_WINDOW_SIZE - 1) {
            self.odf_window[i] = self.odf_window[i + 1];
            self.timestamp_window[i] = self.timestamp_window[i + 1];
            self.loudness_window[i] = self.loudness_window[i + 1];
        }

        self.odf_window[ONSET_WINDOW_SIZE - 1] = odf_val;
        self.timestamp_window[ONSET_WINDOW_SIZE - 1] = timestamp_ms;
        self.loudness_window[ONSET_WINDOW_SIZE - 1] = loudness_dbfs;

        if self.window_count < ONSET_WINDOW_SIZE {
            self.window_count += 1;
            if self.window_count < ONSET_WINDOW_SIZE {
                return None;
            }
        }

        // Mid-point index for local peak picking
        let mid = ONSET_WINDOW_SIZE / 2;
        let candidate_odf = self.odf_window[mid];
        let candidate_ts = self.timestamp_window[mid];
        let candidate_loudness = self.loudness_window[mid];

        // 1. Silence gating
        if candidate_loudness < self.silence_threshold_dbfs {
            return None;
        }

        // 2. Minimum Inter-Onset Interval (MinIOI) debounce
        if candidate_ts <= self.last_onset_timestamp_ms + self.min_ioi_ms {
            return None;
        }

        // 3. Local peak verification (candidate must be >= surrounding window elements)
        let is_local_peak = self
            .odf_window
            .iter()
            .enumerate()
            .all(|(idx, &val)| idx == mid || candidate_odf >= val);

        if !is_local_peak {
            return None;
        }

        // 4. Adaptive thresholding: threshold = median(window) + C * std_dev(window)
        let mut sorted_odf = self.odf_window;
        sorted_odf.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let median = sorted_odf[ONSET_WINDOW_SIZE / 2];

        let mean = self.odf_window.iter().sum::<f32>() / (ONSET_WINDOW_SIZE as f32);
        let variance = self
            .odf_window
            .iter()
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .sum::<f32>()
            / (ONSET_WINDOW_SIZE as f32);
        let std_dev = variance.sqrt();

        let adaptive_threshold = median + self.threshold * std_dev;

        if candidate_odf >= adaptive_threshold && candidate_odf > 0.0 {
            self.last_onset_timestamp_ms = candidate_ts;
            Some(candidate_ts)
        } else {
            None
        }
    }
}
