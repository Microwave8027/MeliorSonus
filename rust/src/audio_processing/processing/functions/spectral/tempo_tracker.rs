/// Pure-Rust Real-Time Tempo and Beat Tracker
/// probably going to deprecate and move to audio analysis
/// Estimates practice tempo (BPM) and emits beat ticks (`is_beat`)
/// using spectral novelty autocorrelation, octave-disambiguating peak picking,
/// human tempo perceptual weighting (log-normal prior), and dynamic phase synchronization.

pub const NOVELTY_HISTORY_LEN: usize = 512;
pub const MIN_BPM: f32 = 40.0;
pub const MAX_BPM: f32 = 250.0;
pub const DEFAULT_TARGET_BPM: f32 = 120.0; // Central human tempo prior

/// Real-time zero-allocation tempo estimator and beat phase tracker.
#[derive(Debug, Clone)]
pub struct TempoTracker {
    sample_rate: f32,
    hop_size: usize,
    frame_rate: f32, // fps = sample_rate / hop_size

    // Novelty history ring buffer
    novelty_ring: [f32; NOVELTY_HISTORY_LEN],
    ring_head: usize,
    total_frames: usize,

    // Spectral flux state for internal novelty calculation
    prev_magnitudes: Vec<f32>,

    // Current tracking state
    bpm: f32,
    confidence: f32,
    beat_interval_frames: f32,
    frames_since_last_beat: f32,
    last_beat_frame: usize,
}

impl TempoTracker {
    /// Creates a new TempoTracker configured for the given audio stream sample rate and hop size.
    pub fn new(sample_rate: f32, hop_size: usize) -> Self {
        let frame_rate = if hop_size > 0 {
            sample_rate / (hop_size as f32)
        } else {
            86.1328
        };
        let initial_interval = (60.0 * frame_rate) / DEFAULT_TARGET_BPM;

        Self {
            sample_rate,
            hop_size,
            frame_rate,
            novelty_ring: [0.0; NOVELTY_HISTORY_LEN],
            ring_head: 0,
            total_frames: 0,
            prev_magnitudes: Vec::new(),
            bpm: DEFAULT_TARGET_BPM,
            confidence: 0.0,
            beat_interval_frames: initial_interval,
            frames_since_last_beat: 0.0,
            last_beat_frame: 0,
        }
    }

    /// Resets all internal histories and state machines.
    pub fn reset(&mut self) {
        self.novelty_ring.fill(0.0);
        self.ring_head = 0;
        self.total_frames = 0;
        self.prev_magnitudes.clear();
        self.bpm = DEFAULT_TARGET_BPM;
        self.confidence = 0.0;
        self.beat_interval_frames = (60.0 * self.frame_rate) / DEFAULT_TARGET_BPM;
        self.frames_since_last_beat = 0.0;
        self.last_beat_frame = 0;
    }

    /// Feeds an FFT magnitude spectrum to compute spectral flux internally and updates tempo/beat tracking.
    ///
    /// Returns `(is_beat, current_bpm, confidence)`.
    pub fn process_magnitude_spectrum(&mut self, magnitudes: &[f32]) -> (bool, f32, f32) {
        if self.prev_magnitudes.len() != magnitudes.len() {
            self.prev_magnitudes.resize(magnitudes.len(), 0.0);
        }

        // Half-wave rectified spectral flux: sum(max(0, X[k] - X_prev[k]))
        let mut flux = 0.0f32;
        for (curr, prev) in magnitudes.iter().zip(self.prev_magnitudes.iter_mut()) {
            let diff = *curr - *prev;
            if diff > 0.0 {
                flux += diff;
            }
            *prev = *curr;
        }

        self.process_novelty_sample(flux)
    }

    /// Feeds a scalar onset novelty value (e.g. from HFC, spectral flux, or energy jump)
    /// and advances the tempo estimation and beat phase tracker.
    ///
    /// Returns `(is_beat, current_bpm, confidence)`.
    pub fn process_novelty_sample(&mut self, novelty: f32) -> (bool, f32, f32) {
        // 1. Store novelty in circular buffer
        self.novelty_ring[self.ring_head] = novelty;
        self.ring_head = (self.ring_head + 1) % NOVELTY_HISTORY_LEN;
        self.total_frames += 1;
        self.frames_since_last_beat += 1.0;

        // Need at least 0.75 seconds of history to start estimating tempo
        let min_required_frames = (self.frame_rate * 0.75).round() as usize;
        if self.total_frames >= min_required_frames {
            // 2. Compute periodic autocorrelation & estimate BPM
            self.estimate_tempo();
        }

        // 3. Phase Alignment & Beat Triggering
        let is_beat = self.check_beat_trigger(novelty);

        (is_beat, self.bpm, self.confidence)
    }

    /// Evaluates autocorrelation across the novelty buffer with human tempo prior and octave disambiguation.
    fn estimate_tempo(&mut self) {
        let history_len = self.total_frames.min(NOVELTY_HISTORY_LEN);

        // Unwrap ring buffer in chronological order into a local array
        let mut unrolled = [0.0f32; NOVELTY_HISTORY_LEN];
        let start_idx = (self.ring_head + NOVELTY_HISTORY_LEN - history_len) % NOVELTY_HISTORY_LEN;
        for i in 0..history_len {
            unrolled[i] = self.novelty_ring[(start_idx + i) % NOVELTY_HISTORY_LEN];
        }

        // Subtract mean
        let mean = unrolled[..history_len].iter().sum::<f32>() / (history_len as f32);
        for x in unrolled[..history_len].iter_mut() {
            *x -= mean;
        }

        // Min and max lag corresponding to BPM range [40, 250]
        let min_lag = ((60.0 * self.frame_rate) / MAX_BPM).round().max(2.0) as usize;
        let max_lag = ((60.0 * self.frame_rate) / MIN_BPM)
            .round()
            .min((history_len / 2) as f32) as usize;

        if min_lag >= max_lag || max_lag >= history_len {
            return;
        }

        let tau_prior = (60.0 * self.frame_rate) / DEFAULT_TARGET_BPM;
        let sigma = 0.8f32; // Broad prior around 120 BPM

        let mut raw_energy = 0.0f32;
        for x in &unrolled[..history_len] {
            raw_energy += x * x;
        }
        let norm_factor = raw_energy.max(1e-6);

        let mut correlations = [0.0f32; NOVELTY_HISTORY_LEN];
        let mut best_score = f32::NEG_INFINITY;
        let mut candidate_lag = min_lag;

        for lag in min_lag..=max_lag {
            let mut sum = 0.0f32;
            let overlap = history_len - lag;
            for n in 0..overlap {
                sum += unrolled[n] * unrolled[n + lag];
            }

            let norm_corr = sum / (norm_factor * ((overlap as f32) / (history_len as f32)));
            let positive_corr = norm_corr.max(0.0);
            correlations[lag] = positive_corr;

            // Log-normal tempo weighting: W(tau) = exp(- (ln(tau / tau_0))^2 / (2 * sigma^2))
            let log_ratio = ((lag as f32) / tau_prior).ln();
            let weight = (-(log_ratio * log_ratio) / (2.0 * sigma * sigma)).exp();

            let score = positive_corr * weight;
            if score > best_score {
                best_score = score;
                candidate_lag = lag;
            }
        }

        // Octave Disambiguation: Check if half-lag (2x BPM) or third-lag has a strong correlation peak
        let mut final_lag = candidate_lag;
        for divisor in [2, 3] {
            let sub_lag = (candidate_lag + (divisor / 2)) / divisor;
            if sub_lag >= min_lag && sub_lag < max_lag {
                let sub_corr = correlations[sub_lag];
                let cand_corr = correlations[candidate_lag];
                // If sub-lag has at least 70% of the candidate's correlation and is a local peak
                let is_local_peak = sub_corr >= correlations[sub_lag.saturating_sub(1)]
                    && sub_corr >= correlations[(sub_lag + 1).min(max_lag)];
                if is_local_peak && sub_corr >= 0.70 * cand_corr && sub_corr > 0.15 {
                    final_lag = sub_lag;
                    break;
                }
            }
        }

        // Parabolic / Quadratic sub-frame interpolation
        let mut interpolated_lag = final_lag as f32;
        if final_lag > min_lag && final_lag < max_lag {
            let alpha = correlations[final_lag - 1];
            let beta = correlations[final_lag];
            let gamma = correlations[final_lag + 1];
            let denom = 2.0 * (alpha - 2.0 * beta + gamma);
            if denom.abs() > 1e-6 {
                let delta = (alpha - gamma) / denom;
                if delta.abs() < 1.0 {
                    interpolated_lag = (final_lag as f32) + delta;
                }
            }
        }

        if interpolated_lag > 0.0 {
            let raw_bpm = (60.0 * self.frame_rate) / interpolated_lag;
            let bounded_bpm = raw_bpm.clamp(MIN_BPM, MAX_BPM);

            // Confidence based on peak correlation
            let raw_confidence = (correlations[final_lag] * 2.0).clamp(0.0, 1.0);

            // Exponential smoothing on BPM
            let alpha_smooth = if self.confidence > 0.30 { 0.20 } else { 0.50 };
            self.bpm = (1.0 - alpha_smooth) * self.bpm + alpha_smooth * bounded_bpm;
            self.confidence =
                (1.0 - alpha_smooth) * self.confidence + alpha_smooth * raw_confidence;
            self.beat_interval_frames = (60.0 * self.frame_rate) / self.bpm;
        }
    }

    /// Checks whether current frame aligns with beat pulse and fires tick.
    fn check_beat_trigger(&mut self, current_novelty: f32) -> bool {
        let period = self.beat_interval_frames;
        if period <= 0.0 {
            return false;
        }

        // Tolerance window around expected beat: +/- 25% of period or at least 2 frames
        let tolerance = (period * 0.25).max(2.0);

        // Has enough time elapsed since the last beat?
        let min_interval = (period * 0.70).max(2.0);
        if self.frames_since_last_beat < min_interval {
            return false;
        }

        // 1. If an onset spike occurred within the anticipation window [period - tolerance, period]
        if self.frames_since_last_beat >= (period - tolerance) {
            let is_onset_spike = current_novelty > 0.5;
            if is_onset_spike {
                self.frames_since_last_beat = 0.0;
                self.last_beat_frame = self.total_frames;
                return true;
            }
        }

        // 2. If we reached or exceeded the expected period without an onset, emit periodic tick
        if self.frames_since_last_beat >= period {
            self.frames_since_last_beat = 0.0;
            self.last_beat_frame = self.total_frames;
            return true;
        }

        false
    }

    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn hop_size(&self) -> usize {
        self.hop_size
    }
}
