# Sound Implementation Plan: Psychoacoustic Critical Band Loudness Model

This document specifies the technical design, mathematical foundation, and production Rust implementation for the **Psychoacoustic Critical Band Loudness Engine (Zwicker Bark Model)**.

---

## 1. Executive Overview

### 1.1 Objective
Implement a high-performance, real-time safe Rust engine for calculating the perceived loudness of polyphonic chords, complex musical signals, and multi-tone acoustic events in **Sones** (linear perceived loudness) and **Phons** (perceived loudness level).

### 1.2 Why This Model for Polyphonic Chords?
Standard broadcast metrics (e.g., ITU-R BS.1770 / LUFS) and physical metrics (RMS / dBFS) evaluate broadband energy linearly:
* **Failure in Polyphony:** When chord voicings change from a tight cluster (e.g., $C_4-D_4-E_4-F_4$) to a spread drop-2 voicing ($C_2-G_3-E_4-C_6$), the physical RMS and LUFS levels remain identical, yet human ears perceive the spread voicing as **up to 2.5× to 3× louder**.
* **Psychoacoustic Solution:** By decomposing signals into **24 Bark critical bands**, applying **asymmetric upward/downward spectral masking**, and applying **non-linear power-law compressive transduction** ($N' \propto E^{0.23}$) *per critical band before summation*, this model matches real human auditory perception.

```
                                      DSP PROCESSING PIPELINE
  
  PCM Audio Frame x[n] (e.g. 2048 samples @ 44.1/48kHz)
            │
            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 1. Windowing: Apply pre-calculated Hann window         │
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 2. Forward FFT: In-place FFT via `rustfft` + scratch   │
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 3. Power Spectrum: P[k] = Re²[k] + Im²[k]              │
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 4. Critical Band Integration: Map to 24 Bark Bands     │
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 5. Asymmetric Excitation Masking Spread:               │
  │    - Upward Masking: -12 dB/Bark (Low roots mask highs)│
  │    - Downward Masking: -27 dB/Bark                     │
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 6. Specific Loudness Transduction (Zwicker α = 0.23):  │
  │    N'[b] = 0.08 · (E/Eth)^0.23 · [(1 + E/Eth)^0.23 - 1]│
  └─────────────────────────┬──────────────────────────────┘
                            │
                            ▼
  ┌────────────────────────────────────────────────────────┐
  │ 7. Total Loudness & Loudness Level Integration:        │
  │    N_total = ∑ N'[b] (Sones)                           │
  │    L_N = 40 + 33.219 · log10(N_total) (Phons)          │
  └────────────────────────────────────────────────────────┘
```

---

## 2. Mathematical Foundation

### 2.1 Bark Frequency Scale (Zwicker Scale)
The mapping between frequency $f$ (in Hz) and critical band number $z$ (in Bark) is governed by:
$$z(f) = 13 \arctan(0.00076 f) + 3.5 \arctan\left(\left(\frac{f}{7500}\right)^2\right) \quad [\text{Bark}]$$

The engine divides the spectrum into 24 standard critical bands spanning $20\text{ Hz}$ to $15.5\text{ kHz}$.

### 2.2 Threshold of Hearing in Quiet ($E_{th}$)
To calibrate quiet thresholds per Bark band, the engine uses the Terhardt auditory threshold curve:
$$T_{q}(f_{\text{kHz}}) = 3.64 \cdot f_{\text{kHz}}^{-0.8} - 6.5 \cdot \exp\left(-0.6 (f_{\text{kHz}} - 3.3)^2\right) + 10^{-3} \cdot f_{\text{kHz}}^4 \quad [\text{dB SPL}]$$
$$E_{th}[b] = 10^{T_q(f_{c,b}) / 10} \cdot 10^{-10}$$

### 2.3 Asymmetric Excitation Spreading (Spectral Masking)
Sound energy in lower frequency bands masks higher frequency partials more aggressively than vice versa:
* **Upward Masking Slope ($s_{\text{up}}$):** $-12\text{ dB/Bark} \implies 10^{-1.2} \approx 0.0631$
* **Downward Masking Slope ($s_{\text{down}}$):** $-27\text{ dB/Bark} \implies 10^{-2.7} \approx 0.0020$

For band $b$:
$$E_{\text{exc}}[b] = E_{\text{band}}[b] + \sum_{k=0}^{b-1} E_{\text{band}}[k] \cdot s_{\text{up}}^{b - k} + \sum_{k=b+1}^{23} E_{\text{band}}[k] \cdot s_{\text{down}}^{k - b}$$

### 2.4 Specific Loudness $N'[b]$ & Total Loudness Integration
Compressive transduction into specific loudness $N'$ (in Sones/Bark):
$$N'[b] = \begin{cases}
0.08 \cdot \left(\frac{E_{\text{exc}}[b]}{E_{th}[b]}\right)^{0.23} \cdot \left[\left(1 + \frac{E_{\text{exc}}[b]}{E_{th}[b]}\right)^{0.23} - 1\right] & \text{if } E_{\text{exc}}[b] > E_{th}[b] \\
0.0 & \text{otherwise}
\end{cases}$$

Total loudness in Sones:
$$N = \sum_{b=0}^{23} N'[b]$$

Conversion to perceived Loudness Level in Phons:
$$L_N = \begin{cases}
40 + \frac{10}{\log_{10}(2)} \log_{10}(N) \approx 40 + 33.21928 \cdot \log_{10}(N) & \text{if } N \ge 1.0 \\
40 \cdot (N + 0.0005)^{0.35} & \text{if } 0 < N < 1.0 \\
0.0 & \text{if } N = 0
\end{cases}$$

---

## 3. Rust Engine Implementation

### 3.1 Crate Setup (`Cargo.toml`)
```toml
[package]
name = "psychoacoustic-loudness"
version = "0.1.0"
edition = "2021"

[dependencies]
rustfft = "6.2"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

### 3.2 Core Implementation (`src/lib.rs`)

```rust
use std::sync::Arc;
use rustfft::{Fft, FftPlanner, num_complex::Complex};

/// Total standard Zwicker critical bands (Bark scale: 1 to 24 Bark)
pub const NUM_BARK_BANDS: usize = 24;

/// Upper frequency boundaries (Hz) for the 24 standard Bark critical bands
pub const BARK_EDGES_HZ: [f32; 25] = [
    20.0, 100.0, 200.0, 300.0, 400.0, 510.0, 630.0, 770.0,
    920.0, 1080.0, 1270.0, 1480.0, 1720.0, 2000.0, 2320.0, 2700.0,
    3150.0, 3700.0, 4400.0, 5300.0, 6400.0, 7700.0, 9500.0, 12000.0, 15500.0,
];

/// Result container holding momentary loudness metrics
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoudnessResult {
    /// Total perceived loudness in Sones (1 Sone = 1 kHz tone @ 40 dB SPL)
    pub sones: f32,
    /// Perceived Loudness Level in Phons
    pub phons: f32,
}

/// Zero-allocation, real-time safe Psychoacoustic Loudness Engine
pub struct PsychoacousticLoudnessMeter {
    fft_size: usize,
    sample_rate: f32,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    bark_bin_ranges: [(usize, usize); NUM_BARK_BANDS],
    hearing_threshold_power: [f32; NUM_BARK_BANDS],
    
    // Pre-allocated scratch buffers to guarantee 0 heap allocations during audio processing
    fft_buffer: Vec<Complex<f32>>,
    scratch_buffer: Vec<Complex<f32>>,
    band_powers: [f32; NUM_BARK_BANDS],
    excitation_pattern: [f32; NUM_BARK_BANDS],
    specific_loudness: [f32; NUM_BARK_BANDS],
}

impl PsychoacousticLoudnessMeter {
    /// Creates a new loudness meter instance.
    ///
    /// # Arguments
    /// * `fft_size` - Size of the analysis window (e.g., 2048 or 4096). Must be a power of 2.
    /// * `sample_rate` - Audio sample rate in Hz (e.g., 44100.0 or 48000.0).
    pub fn new(fft_size: usize, sample_rate: f32) -> Self {
        assert!(fft_size > 0 && (fft_size & (fft_size - 1)) == 0, "fft_size must be a power of two");
        assert!(sample_rate > 0.0, "sample_rate must be positive");

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let scratch_len = fft.get_inplace_scratch_len();

        // 1. Pre-calculate periodic Hann window
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * (i as f32) / (fft_size as f32)).cos())
            })
            .collect();

        // 2. Pre-calculate Bark band FFT bin bounds: [start_bin, end_bin)
        let bin_hz = sample_rate / (fft_size as f32);
        let max_bin = fft_size / 2;
        let mut bark_bin_ranges = [(0, 0); NUM_BARK_BANDS];

        for b in 0..NUM_BARK_BANDS {
            let start_hz = BARK_EDGES_HZ[b];
            let end_hz = BARK_EDGES_HZ[b + 1];
            let start_bin = ((start_hz / bin_hz).floor() as usize).min(max_bin);
            let end_bin = ((end_hz / bin_hz).ceil() as usize).min(max_bin);
            bark_bin_ranges[b] = (start_bin, end_bin.max(start_bin + 1));
        }

        // 3. Pre-calculate threshold of hearing in quiet per Bark band (Terhardt model)
        let mut hearing_threshold_power = [1.0f32; NUM_BARK_BANDS];
        for b in 0..NUM_BARK_BANDS {
            let center_hz = 0.5 * (BARK_EDGES_HZ[b] + BARK_EDGES_HZ[b + 1]);
            let f_khz = (center_hz / 1000.0).max(0.02);
            let t_db = 3.64 * f_khz.powf(-0.8)
                     - 6.5 * (-0.6 * (f_khz - 3.3).powi(2)).exp()
                     + 1e-3 * f_khz.powi(4);
            // Reference linear power
            hearing_threshold_power[b] = 10.0f32.powf(t_db / 10.0) * 1e-9;
        }

        Self {
            fft_size,
            sample_rate,
            fft,
            window,
            bark_bin_ranges,
            hearing_threshold_power,
            fft_buffer: vec![Complex::default(); fft_size],
            scratch_buffer: vec![Complex::default(); scratch_len],
            band_powers: [0.0; NUM_BARK_BANDS],
            excitation_pattern: [0.0; NUM_BARK_BANDS],
            specific_loudness: [0.0; NUM_BARK_BANDS],
        }
    }

    /// Returns the configured FFT size.
    #[inline]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// Returns the configured sample rate.
    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Computes perceived loudness for a single audio frame.
    ///
    /// # Safety & Real-time Guarantees
    /// * Deterministic $\mathcal{O}(N \log N + B^2)$ computational complexity.
    /// * **Zero heap allocations** inside this method.
    /// * Real-time safe for audio callback threads.
    pub fn process_frame(&mut self, samples: &[f32]) -> LoudnessResult {
        assert_eq!(samples.len(), self.fft_size, "Input buffer length must match fft_size");

        // 1. Apply Hann window & populate complex FFT buffer
        for i in 0..self.fft_size {
            self.fft_buffer[i] = Complex {
                re: samples[i] * self.window[i],
                im: 0.0,
            };
        }

        // 2. Perform in-place forward FFT
        self.fft.process_with_scratch(&mut self.fft_buffer, &mut self.scratch_buffer);

        // 3. Integrate power into 24 Bark Critical Bands
        let half_bins = self.fft_size / 2;
        for b in 0..NUM_BARK_BANDS {
            let (start_bin, end_bin) = self.bark_bin_ranges[b];
            let mut sum_power = 0.0f32;
            let upper = end_bin.min(half_bins);
            for k in start_bin..upper {
                sum_power += self.fft_buffer[k].norm_sqr();
            }
            self.band_powers[b] = sum_power;
        }

        // 4. Asymmetric Excitation Masking Spreading
        // Upward masking (lower frequencies mask higher frequencies): -12 dB/Bark
        // Downward masking (higher frequencies weakly mask lower frequencies): -27 dB/Bark
        let s_upward = 0.063095734f32;   // 10^(-1.2)
        let s_downward = 0.001995262f32; // 10^(-2.7)

        for b in 0..NUM_BARK_BANDS {
            let mut excitation = self.band_powers[b];
            
            // Masking contribution from lower bands (Upward spread)
            for k in 0..b {
                let dist = (b - k) as f32;
                excitation += self.band_powers[k] * s_upward.powf(dist);
            }
            // Masking contribution from higher bands (Downward spread)
            for k in (b + 1)..NUM_BARK_BANDS {
                let dist = (k - b) as f32;
                excitation += self.band_powers[k] * s_downward.powf(dist);
            }
            self.excitation_pattern[b] = excitation;
        }

        // 5. Specific Loudness via Compressive Power Law (Zwicker exponent alpha = 0.23)
        let mut total_sones = 0.0f32;
        for b in 0..NUM_BARK_BANDS {
            let eth = self.hearing_threshold_power[b];
            let ratio = (self.excitation_pattern[b] / eth).max(0.0);
            
            let n_prime = if ratio > 1.0 {
                // Compressive specific loudness: N'_b = 0.08 * (E/Eth)^0.23 * [(1 + E/Eth)^0.23 - 1]
                0.08 * ratio.powf(0.23) * ((1.0 + ratio).powf(0.23) - 1.0)
            } else {
                0.0
            };
            
            self.specific_loudness[b] = n_prime;
            total_sones += n_prime;
        }

        // 6. Convert Sones to Phons
        let phons = if total_sones >= 1.0 {
            40.0 + 33.21928 * total_sones.log10()
        } else if total_sones > 0.0 {
            40.0 * (total_sones + 0.0005).powf(0.35)
        } else {
            0.0
        };

        LoudnessResult {
            sones: total_sones,
            phons,
        }
    }

    /// Returns a slice of the most recent specific loudness per Bark band (in Sones/Bark).
    #[inline]
    pub fn specific_loudness(&self) -> &[f32; NUM_BARK_BANDS] {
        &self.specific_loudness
    }
}
```

---

## 4. Verification & Unit Tests

### `src/tests.rs`
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine_wave(freq_hz: f32, sample_rate: f32, num_samples: usize, amplitude: f32) -> Vec<f32> {
        (0..num_samples)
            .map(|i| amplitude * (2.0 * PI * freq_hz * (i as f32) / sample_rate).sin())
            .collect()
    }

    fn calculate_rms(samples: &[f32]) -> f32 {
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    #[test]
    fn test_zero_signal_produces_zero_loudness() {
        let mut meter = PsychoacousticLoudnessMeter::new(2048, 44100.0);
        let silence = vec![0.0f32; 2048];
        let result = meter.process_frame(&silence);
        assert_eq!(result.sones, 0.0);
        assert_eq!(result.phons, 0.0);
    }

    #[test]
    fn test_voicing_loudness_discrepancy() {
        let fft_size = 2048;
        let sample_rate = 44100.0;
        let mut meter = PsychoacousticLoudnessMeter::new(fft_size, sample_rate);

        // 1. Narrow Cluster Voicing: 4 notes clustered tightly inside 1 critical band (~400-480 Hz)
        let cluster_freqs = [400.0, 425.0, 450.0, 475.0];
        let mut cluster_signal = vec![0.0f32; fft_size];
        for &freq in &cluster_freqs {
            let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
            for i in 0..fft_size {
                cluster_signal[i] += tone[i];
            }
        }

        // 2. Open / Spread Voicing: 4 notes distributed across 4 distinct critical bands
        let spread_freqs = [130.81, 329.63, 1046.50, 3135.96];
        let mut spread_signal = vec![0.0f32; fft_size];
        for &freq in &spread_freqs {
            let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
            for i in 0..fft_size {
                spread_signal[i] += tone[i];
            }
        }

        // Normalize both signals to have EXACTLY identical RMS energy
        let rms_cluster = calculate_rms(&cluster_signal);
        let rms_spread = calculate_rms(&spread_signal);
        for s in cluster_signal.iter_mut() { *s /= rms_cluster; }
        for s in spread_signal.iter_mut() { *s /= rms_spread; }

        let result_cluster = meter.process_frame(&cluster_signal);
        let result_spread = meter.process_frame(&spread_signal);

        println!("Cluster Voicing Loudness: {:.2} Sones, {:.2} Phons", result_cluster.sones, result_cluster.phons);
        println!("Spread Voicing Loudness:  {:.2} Sones, {:.2} Phons", result_spread.sones, result_spread.phons);

        assert!(
            result_spread.sones > result_cluster.sones * 1.5,
            "Spread chord should be perceived at least 50% louder than cluster of identical RMS"
        );
    }
}
```

---

## 5. Streaming Integration & Ballistics

```rust
pub struct StreamingLoudnessAnalyzer {
    meter: PsychoacousticLoudnessMeter,
    ring_buffer: Vec<f32>,
    hop_size: usize,
    write_pos: usize,
    samples_since_last_process: usize,
    smoothed_sones: f32,
    smoothing_alpha: f32,
}

impl StreamingLoudnessAnalyzer {
    pub fn new(fft_size: usize, hop_size: usize, sample_rate: f32, attack_time_sec: f32) -> Self {
        let dt = hop_size as f32 / sample_rate;
        let smoothing_alpha = 1.0 - (-dt / attack_time_sec).exp();

        Self {
            meter: PsychoacousticLoudnessMeter::new(fft_size, sample_rate),
            ring_buffer: vec![0.0; fft_size],
            hop_size,
            write_pos: 0,
            samples_since_last_process: 0,
            smoothed_sones: 0.0,
            smoothing_alpha,
        }
    }

    pub fn push_samples(&mut self, incoming: &[f32]) -> Option<LoudnessResult> {
        let mut last_result = None;
        let fft_size = self.meter.fft_size();

        for &sample in incoming {
            self.ring_buffer[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % fft_size;
            self.samples_since_last_process += 1;

            if self.samples_since_last_process >= self.hop_size {
                self.samples_since_last_process = 0;

                let mut frame = vec![0.0f32; fft_size];
                for i in 0..fft_size {
                    let idx = (self.write_pos + i) % fft_size;
                    frame[i] = self.ring_buffer[idx];
                }

                let momentary = self.meter.process_frame(&frame);
                self.smoothed_sones += self.smoothing_alpha * (momentary.sones - self.smoothed_sones);
                
                let smoothed_phons = if self.smoothed_sones >= 1.0 {
                    40.0 + 33.21928 * self.smoothed_sones.log10()
                } else if self.smoothed_sones > 0.0 {
                    40.0 * (self.smoothed_sones + 0.0005).powf(0.35)
                } else {
                    0.0
                };

                last_result = Some(LoudnessResult {
                    sones: self.smoothed_sones,
                    phons: smoothed_phons,
                });
            }
        }

        last_result
    }
}
```
## IMPORTANT 
**AVOID DYNAMIC HEAP ALLOCATIONS AS MUCH AS POSSIBLE UNLESS NOT POSSIBLE**
---

## 6. Real-Time Safety & Optimization Checklist

1. **Denormal Subnormal Protection:**
   Enable Flush-To-Zero (`FTZ`) and Denormals-Are-Zero (`DAZ`) on the audio processing thread:
   ```rust
   #[cfg(target_arch = "x86_64")]
   unsafe {
       use std::arch::x86_64::*;
       _MM_SET_FLUSH_ZERO_MODE(_MM_FLUSH_ZERO_ON);
       _MM_SET_DENORMALS_ZERO_MODE(_MM_DENORMALS_ZERO_ON);
   }
   ```
2. **SIMD Target Flag:**
   Enable native instruction compilation in `.cargo/config.toml`:
   ```toml
   [build]
   rustflags = ["-C", "target-cpu=native"]
   ```
3. **Threading Model:**
   Process FFT frames on dedicated background worker threads or real-time audio threads; avoid blocking the main UI dispatcher.
   use cpu affinity to process the fft on a dedicated core to prevent starving the background audio and to require maximum needed computation on one core if the target device is android, linux, or windows. It doesnt support ios so just dont worry about that for ios. 
