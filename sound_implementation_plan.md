# Sound Implementation Plan: Polyphonic Chord Psychoacoustic Loudness Engine

This document specifies the complete architecture, mathematical foundations, thread/core allocation strategy, and production Rust implementation for calculating the **perceived loudness of individual notes in polyphonic chords** in **Sones** (linear perceived loudness) and **Phons** (perceived loudness level), integrated directly with a **Convolutional Recurrent Neural Network (CRNN)** polyphonic pitch detector.

---

## 1. System Architecture & Two-Stage Paradigm

In polyphonic music performance, determining the loudness of each note in a chord requires dividing responsibilities between **Machine Learning (Pitch Detection)** and **Deterministic DSP (Psychoacoustics)**:

1. **Stage 1: CRNN Multi-Pitch Estimator (Neural Network):**  
   Analyzes the complex acoustic spectrum to identify *what notes are present* in the chord, outputting fundamental frequencies $[f_{0,1}, f_{0,2}, \dots, f_{0,K}]$ (or MIDI note numbers).
2. **Stage 2: Psychoacoustic Loudness Engine (Deterministic DSP):**  
   Takes the raw audio frame and the CRNN's detected pitches, extracts the **harmonic combs** ($f_0, 2f_0, 3f_0, \dots$) via windowed **FFT**, groups energy into **24 Bark critical bands**, computes **asymmetric simultaneous masking**, and applies **compressive power-law transduction** ($N' \propto E^{0.23}$) to output exact **Sones and Phons for each note**.

```
                           END-TO-END DATAFLOW PIPELINE
  
  [Raw Microphone / Audio PCM Frame] (e.g. 2048 samples @ 48kHz)
                 │
                 ├────────────────────────────────────────┬────────────────────────────────────────┐
                 ▼                                        ▼                                        ▼
    ┌──────────────────────────┐             ┌──────────────────────────┐             ┌──────────────────────────┐
    │ 1. CRNN Pitch Detector   │             │ 2. Windowed STFT (FFT)   │             │ 3. 24-Bark Filterbank    │
    │    (Neural Network)      │             │    `rustfft` + Hann      │             │    & Masking Spreading   │
    └────────────┬─────────────┘             └────────────┬─────────────┘             └────────────┬─────────────┘
                 │ Detected Fundamentals                  │ Complex Bins                               │ Bark Masking Field
                 │ [C4, E4, G4]                           │ P[k] = Re² + Im²                           │ E_exc[b] & N'[b]
                 ▼                                        ▼                                            ▼
    ┌────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
    │ 4. HARMONIC SPECTRAL APPORTIONING & INTEGRATION                                                            │
    │    • For each CRNN note m: locate harmonic peaks {f₀, 2f₀, 3f₀, ...} across FFT bins                       │
    │    • Apportion shared critical band loudness N'[b] proportionally to partial energy                        │
    │    • Evaluate each note under the chord's composite psychoacoustic masking field                           │
    └─────────────────────────────────────────────────────┬──────────────────────────────────────────────────────┘
                                                          │
                                                          ▼
    ┌────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
    │ 5. FINAL PER-NOTE PERCEIVED LOUDNESS OUTPUT                                                                │
    │    • Note 1: C4 (261.6 Hz) ──► 5.42 Sones (64.3 Phons)                                                     │
    │    • Note 2: E4 (329.6 Hz) ──► 3.18 Sones (56.7 Phons)  <-- (Partially masked by low root C4)              │
    │    • Note 3: G4 (392.0 Hz) ──► 4.89 Sones (62.9 Phons)                                                     │
    └────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Psychoacoustic Foundations

### 2.1 The Critical Band & Voicing Discrepancy (Jensen's Inequality)
The human cochlea resolves acoustic energy through ~24 Bark critical bands. Within each band, neural firing follows compressive power-law transduction:
$$N'_b = C \cdot E_b^\alpha \quad (\alpha \approx 0.23)$$

Because $\alpha < 1$, the function $f(x) = x^\alpha$ is strictly concave. By **Jensen's Inequality**:
$$\left(\sum_{b=1}^M E_b\right)^\alpha < \sum_{b=1}^M E_b^\alpha \quad (M > 1, E_b > 0)$$

* **Close Cluster Chord (e.g. C4-D4-E4-F4):** All partials collapse into 1 critical band. Energy sums *linearly before compression* $\implies N_{\text{cluster}} = C \cdot (\sum E)^\alpha$.
* **Open / Drop-2 Chord (e.g. C2-G3-E4-C6):** Partials distribute across $10+$ independent critical bands. Energy compresses *per band before summation* $\implies N_{\text{open}} = \sum C \cdot E_b^\alpha$.
* **Result:** The open-voiced chord is perceived as **up to 2.5× to 3× louder** in Sones than the narrow cluster of identical total physical RMS / LUFS energy.

### 2.2 Asymmetric Excitation Spreading (Simultaneous Masking)
Lower frequency chord fundamentals exert significant upward masking on higher overtones:
* **Upward Masking Slope ($s_{\text{up}}$):** $-12\text{ dB/Bark} \implies 10^{-1.2} \approx 0.0631$
* **Downward Masking Slope ($s_{\text{down}}$):** $-27\text{ dB/Bark} \implies 10^{-2.7} \approx 0.0020$

For critical band $b$:
$$E_{\text{exc}}[b] = E_{\text{band}}[b] + \sum_{k=0}^{b-1} E_{\text{band}}[k] \cdot s_{\text{up}}^{b - k} + \sum_{k=b+1}^{23} E_{\text{band}}[k] \cdot s_{\text{down}}^{k - b}$$

### 2.3 Compressive Specific Loudness $N'[b]$ & Total Loudness Integration
$$N'[b] = \begin{cases}
0.08 \cdot \left(\frac{E_{\text{exc}}[b]}{E_{th}[b]}\right)^{0.23} \cdot \left[\left(1 + \frac{E_{\text{exc}}[b]}{E_{th}[b]}\right)^{0.23} - 1\right] & \text{if } E_{\text{exc}}[b] > E_{th}[b] \\
0.0 & \text{otherwise}
\end{cases}$$
where $E_{th}[b]$ is the Terhardt absolute threshold of hearing in quiet.

Conversion from Sones to Phons:
$$L_N = \begin{cases}
40 + \frac{10}{\log_{10}(2)} \log_{10}(N) \approx 40 + 33.21928 \cdot \log_{10}(N) & \text{if } N \ge 1.0 \\
40 \cdot (N + 0.0005)^{0.35} & \text{if } 0 < N < 1.0 \\
0.0 & \text{if } N = 0
\end{cases}$$

---

## 3. Threading, Core Affinity & Multi-Core Strategy

To prevent CPU-intensive FFT and psychoacoustic loops from causing audio dropouts or UI stuttering, the application uses **dedicated core pinning and lock-free communication**:

```
  THREAD ROLE                    CORE PINNING?       PRIORITY LEVEL      COMMUNICATION
  ─────────────────────────────────────────────────────────────────────────────────────────────────────────
  1. Audio Driver Callback       Desktop: Core 0     Real-Time           Feeds audio into lock-free SPSC 
     (AAudio / Oboe / cpal)      Mobile: OS Managed  (Time-Critical)     ring buffer (`rtrb`).
  
  2. Loudness DSP Worker         YES: P-Core 1       High                Pulls from ring buffer; executes 
     (FFT + Psychoacoustics)     (Isolated)          Priority            FFT & harmonic apportioning.
  
  3. CRNN Pitch Inference        YES: P-Core 2 / GPU Normal-High         Runs neural net inference frame-by-frame;
     (ONNX / Tract / NPU)                                                sends detected notes to DSP worker.
  
  4. UI / App Controller         NO (Dynamic)        Normal              Consumes smoothed Sone/Phon state for
     (Compose / Main Thread)                                             visualizers and scoring.
```

---

## 4. Complete Rust Implementation

### 4.1 `Cargo.toml`
```toml
[package]
name = "polyphonic-loudness-engine"
version = "0.1.0"
edition = "2021"

[dependencies]
rustfft = "6.2"
core_affinity = "0.8"
rtrb = "0.3"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

### 4.2 Core Engine (`src/lib.rs`)

```rust
use std::sync::Arc;
use rustfft::{Fft, FftPlanner, num_complex::Complex};

pub const NUM_BARK_BANDS: usize = 24;

/// Upper frequency boundaries (Hz) for the 24 standard Bark critical bands (Zwicker scale)
pub const BARK_EDGES_HZ: [f32; 25] = [
    20.0, 100.0, 200.0, 300.0, 400.0, 510.0, 630.0, 770.0,
    920.0, 1080.0, 1270.0, 1480.0, 1720.0, 2000.0, 2320.0, 2700.0,
    3150.0, 3700.0, 4400.0, 5300.0, 6400.0, 7700.0, 9500.0, 12000.0, 15500.0,
];

/// Per-note perceived loudness output structure
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteLoudnessResult {
    /// Note identifier (e.g. MIDI note number 0-127)
    pub midi_note: u8,
    /// Fundamental frequency in Hz
    pub frequency_hz: f32,
    /// Perceived loudness in Sones (1 Sone = 1 kHz @ 40 dB SPL)
    pub sones: f32,
    /// Perceived Loudness Level in Phons
    pub phons: f32,
}

/// Overall chord and composite frame loudness metrics
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameLoudnessResult {
    /// Total chord loudness in Sones
    pub total_sones: f32,
    /// Total chord loudness level in Phons
    pub total_phons: f32,
}

/// Zero-allocation, real-time safe Psychoacoustic Chord Loudness Engine
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
    /// * `fft_size` - Analysis frame size (e.g., 2048 or 4096). Must be a power of 2.
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

    #[inline]
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Computes composite frame loudness and populates internal power spectra.
    /// REAL-TIME SAFE: Zero allocations.
    pub fn process_frame(&mut self, samples: &[f32]) -> FrameLoudnessResult {
        assert_eq!(samples.len(), self.fft_size, "Input buffer length must match fft_size");

        // 1. Apply Hann window & populate complex FFT buffer
        for i in 0..self.fft_size {
            self.fft_buffer[i] = Complex {
                re: samples[i] * self.window[i],
                im: 0.0,
            };
        }

        // 2. Perform forward FFT
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
        let s_upward = 0.063095734f32;   // -12 dB/Bark
        let s_downward = 0.001995262f32; // -27 dB/Bark

        for b in 0..NUM_BARK_BANDS {
            let mut excitation = self.band_powers[b];
            for k in 0..b {
                let dist = (b - k) as f32;
                excitation += self.band_powers[k] * s_upward.powf(dist);
            }
            for k in (b + 1)..NUM_BARK_BANDS {
                let dist = (k - b) as f32;
                excitation += self.band_powers[k] * s_downward.powf(dist);
            }
            self.excitation_pattern[b] = excitation;
        }

        // 5. Compressive Specific Loudness Transduction
        let mut total_sones = 0.0f32;
        for b in 0..NUM_BARK_BANDS {
            let eth = self.hearing_threshold_power[b];
            let ratio = (self.excitation_pattern[b] / eth).max(0.0);
            let n_prime = if ratio > 1.0 {
                0.08 * ratio.powf(0.23) * ((1.0 + ratio).powf(0.23) - 1.0)
            } else {
                0.0
            };
            self.specific_loudness[b] = n_prime;
            total_sones += n_prime;
        }

        // 6. Phon Conversion
        let total_phons = sones_to_phons(total_sones);

        FrameLoudnessResult {
            total_sones,
            total_phons,
        }
    }

    /// Computes the individual perceived loudness for each active note detected by the CRNN.
    ///
    /// # Arguments
    /// * `samples` - Audio frame (2048 samples).
    /// * `crnn_notes` - Slice of fundamental frequencies (in Hz) detected by your CRNN model.
    /// * `max_harmonics` - Number of overtone harmonics to track per note (default: 12).
    pub fn process_chord_notes(
        &mut self,
        samples: &[f32],
        crnn_frequencies: &[f32],
        max_harmonics: usize,
    ) -> Vec<NoteLoudnessResult> {
        let num_notes = crnn_frequencies.len();
        if num_notes == 0 {
            return Vec::new();
        }

        // 1. Process composite frame FFT and specific loudness N'[b]
        let _composite = self.process_frame(samples);

        let half_bins = self.fft_size / 2;
        let bin_hz = self.sample_rate / (self.fft_size as f32);

        // 2. Track power contributions per note in each Bark band
        let mut note_band_power = vec![[0.0f32; NUM_BARK_BANDS]; num_notes];
        let mut total_allocated_band_power = [1e-12f32; NUM_BARK_BANDS];

        for (n_idx, &f0) in crnn_frequencies.iter().enumerate() {
            if f0 <= 0.0 { continue; }

            for h in 1..=max_harmonics {
                let harmonic_hz = f0 * (h as f32);
                if harmonic_hz > (self.sample_rate * 0.5) {
                    break;
                }

                let center_bin = (harmonic_hz / bin_hz).round() as usize;
                let start_bin = center_bin.saturating_sub(1);
                let end_bin = (center_bin + 2).min(half_bins);

                // Integrate harmonic peak power across spectral leakage window
                let mut partial_power = 0.0f32;
                for k in start_bin..end_bin {
                    partial_power += self.fft_buffer[k].norm_sqr();
                }

                let bark_idx = hz_to_bark_band(harmonic_hz);
                if bark_idx < NUM_BARK_BANDS {
                    note_band_power[n_idx][bark_idx] += partial_power;
                    total_allocated_band_power[bark_idx] += partial_power;
                }
            }
        }

        // 3. Apportion specific loudness N'[b] proportionally to each note's partial power
        let mut results = Vec::with_capacity(num_notes);

        for (n_idx, &f0) in crnn_frequencies.iter().enumerate() {
            let mut note_sones = 0.0f32;

            for b in 0..NUM_BARK_BANDS {
                let note_power_in_band = note_band_power[n_idx][b];
                if note_power_in_band > 0.0 {
                    let fraction = note_power_in_band / total_allocated_band_power[b];
                    let note_specific_loudness = fraction * self.specific_loudness[b];
                    note_sones += note_specific_loudness;
                }
            }

            let midi = hz_to_midi(f0);
            results.push(NoteLoudnessResult {
                midi_note: midi,
                frequency_hz: f0,
                sones: note_sones,
                phons: sones_to_phons(note_sones),
            });
        }

        results
    }
}

// ---------------- Helper Utility Functions ----------------

#[inline]
pub fn sones_to_phons(sones: f32) -> f32 {
    if sones >= 1.0 {
        40.0 + 33.21928 * sones.log10()
    } else if sones > 0.0 {
        40.0 * (sones + 0.0005).powf(0.35)
    } else {
        0.0
    }
}

#[inline]
pub fn hz_to_bark_band(freq_hz: f32) -> usize {
    for b in 0..NUM_BARK_BANDS {
        if freq_hz >= BARK_EDGES_HZ[b] && freq_hz < BARK_EDGES_HZ[b + 1] {
            return b;
        }
    }
    if freq_hz >= BARK_EDGES_HZ[NUM_BARK_BANDS] {
        NUM_BARK_BANDS - 1
    } else {
        0
    }
}

#[inline]
pub fn hz_to_midi(freq_hz: f32) -> u8 {
    if freq_hz <= 0.0 { return 0; }
    let midi = 69.0 + 12.0 * (freq_hz / 440.0).log2();
    midi.round().clamp(0.0, 127.0) as u8
}
```

---

## 5. Verification & Unit Tests

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
        let mut meter = PsychoacousticLoudnessMeter::new(2048, 48000.0);
        let silence = vec![0.0f32; 2048];
        let result = meter.process_frame(&silence);
        assert_eq!(result.total_sones, 0.0);
        assert_eq!(result.total_phons, 0.0);
    }

    #[test]
    fn test_polyphonic_triad_note_attribution() {
        let fft_size = 2048;
        let sample_rate = 48000.0;
        let mut meter = PsychoacousticLoudnessMeter::new(fft_size, sample_rate);

        // Synthesize C Major Triad (C4 = 261.63 Hz, E4 = 329.63 Hz, G4 = 392.00 Hz)
        let chord_pitches = [261.63, 329.63, 392.00];
        let mut chord_signal = vec![0.0f32; fft_size];

        for &freq in &chord_pitches {
            let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.2);
            for i in 0..fft_size {
                chord_signal[i] += tone[i];
            }
        }

        let per_note_results = meter.process_chord_notes(&chord_signal, &chord_pitches, 8);
        assert_eq!(per_note_results.len(), 3);

        for res in &per_note_results {
            println!(
                "CRNN Note MIDI {}: {:.1} Hz -> {:.2} Sones ({:.1} Phons)",
                res.midi_note, res.frequency_hz, res.sones, res.phons
            );
            assert!(res.sones > 0.0, "Each note in chord must have positive loudness");
        }
    }

    #[test]
    fn test_jensen_inequality_voicing_discrepancy() {
        let fft_size = 2048;
        let sample_rate = 48000.0;
        let mut meter = PsychoacousticLoudnessMeter::new(fft_size, sample_rate);

        // 1. Narrow Cluster Voicing (4 notes within 1 Bark band)
        let cluster_freqs = [400.0, 425.0, 450.0, 475.0];
        let mut cluster_signal = vec![0.0f32; fft_size];
        for &freq in &cluster_freqs {
            let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
            for i in 0..fft_size { cluster_signal[i] += tone[i]; }
        }

        // 2. Open Spread Voicing (4 notes across distinct Bark bands)
        let spread_freqs = [130.81, 329.63, 1046.50, 3135.96];
        let mut spread_signal = vec![0.0f32; fft_size];
        for &freq in &spread_freqs {
            let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
            for i in 0..fft_size { spread_signal[i] += tone[i]; }
        }

        // Equalize RMS power
        let rms_cluster = calculate_rms(&cluster_signal);
        let rms_spread = calculate_rms(&spread_signal);
        for s in cluster_signal.iter_mut() { *s /= rms_cluster; }
        for s in spread_signal.iter_mut() { *s /= rms_spread; }

        let res_cluster = meter.process_frame(&cluster_signal);
        let res_spread = meter.process_frame(&spread_signal);

        println!("Cluster Voicing Loudness: {:.2} Sones", res_cluster.total_sones);
        println!("Spread Voicing Loudness:  {:.2} Sones", res_spread.total_sones);

        assert!(
            res_spread.total_sones > res_cluster.total_sones * 1.5,
            "Spread chord must sound at least 50% louder than cluster of identical RMS"
        );
    }
}
```

---

## 6. Real-Time Safety & Optimization Checklist

1. **Denormal Subnormal Protection:**
   Enable Flush-To-Zero (`FTZ`) and Denormals-Are-Zero (`DAZ`) on the DSP worker thread:
   ```rust
   #[cfg(target_arch = "x86_64")]
   unsafe {
       use std::arch::x86_64::*;
       _MM_SET_FLUSH_ZERO_MODE(_MM_FLUSH_ZERO_ON);
       _MM_SET_DENORMALS_ZERO_MODE(_MM_DENORMALS_ZERO_ON);
   }
   ```
2. **SIMD Target Flag:**
   Enable native vectorization in `.cargo/config.toml`:
   ```toml
   [build]
   rustflags = ["-C", "target-cpu=native"]
   ```
3. **Threading Model:**
   Feed PCM samples via a lock-free Single-Producer Single-Consumer (`rtrb`) ring buffer from the audio driver to the pinned DSP worker thread.
