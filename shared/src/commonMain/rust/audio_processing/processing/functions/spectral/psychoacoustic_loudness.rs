use crate::audio_processing::instruments::notes::Notes;
use crate::audio_processing::neural::crnn::NUM_MIDI_NOTES;
use crate::constants::FRAME_SIZE;
use arrayvec::ArrayVec;
use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};
use std::f32::consts::PI;
use std::sync::Arc;

/// Total standard Zwicker critical bands (Bark scale: 1 to 24 Bark).
pub const NUM_BARK_BANDS: usize = 24;

/// Maximum simultaneous chord notes supported on the pre-allocated scratchpad (covering all MIDI notes).
pub const MAX_LOUDNESS_NOTES: usize = NUM_MIDI_NOTES;

pub const MAX_HARMONICS: usize = 12;

/// Upper frequency boundaries (Hz) for the 24 standard Bark critical bands (Zwicker scale).
pub const BARK_EDGES_HZ: [f32; 25] = [
    20.0, 100.0, 200.0, 300.0, 400.0, 510.0, 630.0, 770.0, 920.0, 1080.0, 1270.0, 1480.0, 1720.0,
    2000.0, 2320.0, 2700.0, 3150.0, 3700.0, 4400.0, 5300.0, 6400.0, 7700.0, 9500.0, 12000.0,
    15500.0,
];

/// Type alias mapping frequency to perceived loudness in Sones.
pub type FrequencyLoudnessMap = ArrayVec<(f32, f32), MAX_LOUDNESS_NOTES>;

/// Explicit per-pitch perceived loudness structure.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PitchLoudness {
    /// Perceived Loudness Level in Phons.
    pub phons: f32,
    /// Perceived loudness in Sones (1 Sone = 1 kHz tone @ 40 dB SPL).
    pub sones: f32,
}

/// Per-note perceived loudness output structure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteLoudnessResult {
    /// Note identifier (MIDI note number 0-127).
    pub midi_note: u8,
    /// Fundamental frequency in Hz.
    pub frequency_hz: f32,
    /// Perceived loudness in Sones (1 Sone = 1 kHz tone @ 40 dB SPL).
    pub sones: f32,
    /// Perceived Loudness Level in Phons.
    pub phons: f32,
}

/// Overall chord and composite frame loudness metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameLoudnessResult {
    /// Total composite frame loudness in Sones.
    pub total_sones: f32,
    /// Total composite frame loudness level in Phons.
    pub total_phons: f32,
}

/// Zero-allocation, real-time safe Psychoacoustic Loudness Engine based on the Zwicker Bark Model.
///
/// Generic over analysis frame size `const N: usize` (defaults to `FRAME_SIZE = 1024`).
///
/// Features:
/// 24 Bark critical band decomposition (Zwicker scale).
/// Fixed-size Hann window array `[f32; N]` (0 heap allocations).
/// Asymmetric upward (-12 dB/Bark) and downward (-27 dB/Bark) excitation masking spreading.
/// Compressive power-law specific loudness transduction ($N' \propto E^{0.23}$).
/// Harmonic spectral peak apportioning with pre-allocated scratchpad for polyphonic chords.
pub struct PsychoacousticLoudnessMeter<const N: usize = FRAME_SIZE> {
    sample_rate: f32,
    fft: Arc<dyn RealToComplex<f32>>,
    window: [f32; N],
    bark_bin_ranges: [(usize, usize); NUM_BARK_BANDS],
    hearing_threshold_power: [f32; NUM_BARK_BANDS],
    upward_spread: [f32; NUM_BARK_BANDS],
    downward_spread: [f32; NUM_BARK_BANDS],

    // Pre-allocated scratch buffers to guarantee 0 heap allocations during audio processing loops
    input_buf: Vec<f32>,
    output_buf: Vec<Complex32>,
    scratch_buf: Vec<Complex32>,
    band_powers: [f32; NUM_BARK_BANDS],
    excitation_pattern: [f32; NUM_BARK_BANDS],
    specific_loudness: [f32; NUM_BARK_BANDS],
    scratch_note_band_power: [[f32; NUM_BARK_BANDS]; MAX_LOUDNESS_NOTES],
}

impl<const N: usize> PsychoacousticLoudnessMeter<N> {
    /// Creates a new loudness meter instance.
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz (e.g., 44100.0 or 48000.0).
    pub fn new(u32_sample_rate: u32) -> Self {
        let sample_rate = u32_sample_rate as f32;
        assert!(
            N > 0 && (N & (N - 1)) == 0,
            "N (FFT size) must be a power of two"
        );
        assert!(sample_rate > 0.0, "sample_rate must be positive");

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(N);
        let input_buf = fft.make_input_vec();
        let output_buf = fft.make_output_vec();
        let scratch_buf = fft.make_scratch_vec();

        // 1. Pre-calculate periodic Hann window directly in fixed array: 0 heap allocations!
        let mut window = [0.0f32; N];
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 * (1.0 - (2.0 * PI * (i as f32) / (N as f32)).cos());
        }

        // 2. Pre-calculate Bark band FFT bin bounds: [start_bin, end_bin), excluding DC (bin 0)
        let bin_hz = sample_rate / (N as f32);
        let max_bin = N / 2;
        let mut bark_bin_ranges = [(0, 0); NUM_BARK_BANDS];

        let edge_bins: [usize; 25] = core::array::from_fn(|i| {
            ((BARK_EDGES_HZ[i] / bin_hz).round() as usize).clamp(1, max_bin)
        });

        for b in 0..NUM_BARK_BANDS {
            let start = edge_bins[b];
            let end = edge_bins[b + 1].max(start + 1);
            bark_bin_ranges[b] = (start, end);
        }

        // 3. Pre-calculate threshold of hearing in quiet per Bark band (Terhardt model calibrated to 94 dB SPL @ 0 dBFS)
        let mut hearing_threshold_power = [1.0f32; NUM_BARK_BANDS];
        for b in 0..NUM_BARK_BANDS {
            let center_hz = 0.5 * (BARK_EDGES_HZ[b] + BARK_EDGES_HZ[b + 1]);
            let f_khz = (center_hz / 1000.0).max(0.02);
            let t_db = 3.64 * f_khz.powf(-0.8) - 6.5 * (-0.6 * (f_khz - 3.3).powi(2)).exp()
                + 1e-3 * f_khz.powi(4);
            // Reference linear power calibrated so 0 dBFS corresponds to ~94 dB SPL
            hearing_threshold_power[b] = 10.0f32.powf((t_db - 94.0) / 10.0);
        }

        // 4. Precompute upward (-12 dB/Bark) and downward (-27 dB/Bark) masking spread decay factors
        let s_upward = 0.063095734f32; // 10^(-1.2)
        let s_downward = 0.001995262f32; // 10^(-2.7)

        let mut upward_spread = [0.0f32; NUM_BARK_BANDS];
        let mut downward_spread = [0.0f32; NUM_BARK_BANDS];
        for d in 0..NUM_BARK_BANDS {
            upward_spread[d] = s_upward.powf(d as f32);
            downward_spread[d] = s_downward.powf(d as f32);
        }

        Self {
            sample_rate,
            fft,
            window,
            bark_bin_ranges,
            hearing_threshold_power,
            upward_spread,
            downward_spread,
            input_buf,
            output_buf,
            scratch_buf,
            band_powers: [0.0; NUM_BARK_BANDS],
            excitation_pattern: [0.0; NUM_BARK_BANDS],
            specific_loudness: [0.0; NUM_BARK_BANDS],
            scratch_note_band_power: [[0.0; NUM_BARK_BANDS]; MAX_LOUDNESS_NOTES],
        }
    }

    #[inline]
    pub fn fft_size(&self) -> usize {
        N
    }

    #[inline]
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    #[inline]
    pub fn specific_loudness(&self) -> &[f32; NUM_BARK_BANDS] {
        &self.specific_loudness
    }

    /// Computes composite frame loudness and populates internal power spectra.
    ///
    /// # Safety & Real-time Guarantees
    /// * Deterministic $\mathcal{O}(N \log N + B^2)$ complexity.
    /// * **Zero heap allocations** inside this method.
    pub fn process_frame(&mut self, samples: &[f32]) -> FrameLoudnessResult {
        assert_eq!(
            samples.len(),
            N,
            "Input buffer length ({}) must match FFT size ({})",
            samples.len(),
            N
        );

        // 1. Apply Hann window into pre-allocated real input buffer
        for i in 0..N {
            self.input_buf[i] = samples[i] * self.window[i];
        }

        // 2. Perform forward SIMD real-to-complex FFT
        let _ = self.fft.process_with_scratch(
            &mut self.input_buf,
            &mut self.output_buf,
            &mut self.scratch_buf,
        );

        // 3. Integrate power into 24 Bark Critical Bands (normalized by N^2)
        let norm = 1.0 / (N as f32);
        let half_bins = N / 2;
        for b in 0..NUM_BARK_BANDS {
            let (start_bin, end_bin) = self.bark_bin_ranges[b];
            let mut sum_power = 0.0f32;
            let upper = end_bin.min(half_bins);
            for k in start_bin..upper {
                let c = self.output_buf[k];
                let re = c.re * norm;
                let im = c.im * norm;
                sum_power += re * re + im * im;
            }
            self.band_powers[b] = sum_power;
        }

        // 4. Asymmetric Excitation Masking Spreading
        for b in 0..NUM_BARK_BANDS {
            let mut excitation = self.band_powers[b];
            for k in 0..b {
                excitation += self.band_powers[k] * self.upward_spread[b - k];
            }
            for k in (b + 1)..NUM_BARK_BANDS {
                excitation += self.band_powers[k] * self.downward_spread[k - b];
            }
            self.excitation_pattern[b] = excitation;
        }

        // 5. Compressive Specific Loudness Transduction (Zwicker exponent alpha = 0.23)
        let mut total_sones = 0.0f32;
        for b in 0..NUM_BARK_BANDS {
            let eth = self.hearing_threshold_power[b];
            let ratio = (self.excitation_pattern[b] / eth).max(0.0);
            let n_prime = if ratio > 1.0 {
                0.08 * ((1.0 + ratio).powf(0.23) - 1.0)
            } else {
                0.0
            };
            self.specific_loudness[b] = n_prime;
            total_sones += n_prime;
        }

        let total_phons = sones_to_phons(total_sones);

        FrameLoudnessResult {
            total_sones,
            total_phons,
        }
    }

    /// Calculates perceived loudness in (phons, sones) for explicit fundamental frequencies.
    ///
    /// Returns an array containing perceived loudness for midi notes 0..127,
    /// where active chord notes contain `Some(PitchLoudness)` and inactive notes contain `None`.
    pub fn calculate_frequency_loudness(
        &mut self,
        samples: &[f32],
        frequencies: &[f32],
    ) -> [Option<PitchLoudness>; NUM_MIDI_NOTES] {
        let mut arr = [None; NUM_MIDI_NOTES];
        if frequencies.is_empty() {
            return arr;
        }

        let results = self.process_chord_notes(samples, frequencies, MAX_HARMONICS);
        for res in results {
            let midi = res.midi_note as usize;
            if let Some(val) = arr.get_mut(midi) {
                *val = Some(PitchLoudness {
                    phons: res.phons,
                    sones: res.sones,
                });
            }
        }
        arr
    }

    /// Evaluates individual perceived loudness for active chord notes using harmonic peak apportioning.
    ///
    /// Uses pre-allocated scratchpad memory (`scratch_note_band_power`) to ensure 0 per-frame heap allocations.
    ///
    /// # Arguments
    /// * `samples` - Audio PCM frame slice (length must match `N`).
    /// * `frequencies` - Slice of fundamental frequencies in Hz.
    /// * `max_harmonics` - Maximum overtone harmonics to evaluate per fundamental (default: 12).
    pub fn process_chord_notes(
        &mut self,
        samples: &[f32],
        frequencies: &[f32],
        max_harmonics: usize,
    ) -> ArrayVec<NoteLoudnessResult, MAX_LOUDNESS_NOTES> {
        let num_notes = frequencies.len().min(MAX_LOUDNESS_NOTES);
        if num_notes == 0 {
            return ArrayVec::new();
        }

        // 1. Process composite frame FFT and update specific loudness N'[b]
        let _composite = self.process_frame(samples);

        let bin_hz = self.sample_rate / (N as f32);
        let norm = 1.0 / (N as f32);

        // 2. Track power contributions per note using pre-allocated scratch memory (0 heap allocations!)
        for n in 0..num_notes {
            self.scratch_note_band_power[n] = [0.0; NUM_BARK_BANDS];
        }
        let mut total_allocated_band_power = [1e-12f32; NUM_BARK_BANDS];

        for (n_idx, &f0) in frequencies[..num_notes].iter().enumerate() {
            if f0 <= 0.0 || f0.is_nan() || f0.is_infinite() {
                continue;
            }

            for h in 1..=max_harmonics {
                let harmonic_hz = f0 * (h as f32);
                if harmonic_hz > (self.sample_rate * 0.5) {
                    break;
                }

                let center_bin = (harmonic_hz / bin_hz).round() as usize;
                let start_bin = center_bin.saturating_sub(1);
                let end_bin = (center_bin + 2).min(self.output_buf.len());

                // Integrate harmonic peak power across spectral leakage window (normalized by N^2)
                let mut partial_power = 0.0f32;
                for k in start_bin..end_bin {
                    let c = self.output_buf[k];
                    let re = c.re * norm;
                    let im = c.im * norm;
                    partial_power += re * re + im * im;
                }

                let bark_idx = hz_to_bark_band(harmonic_hz);
                if bark_idx < NUM_BARK_BANDS {
                    self.scratch_note_band_power[n_idx][bark_idx] += partial_power;
                    total_allocated_band_power[bark_idx] += partial_power;
                }
            }
        }

        // 3. Apportion specific loudness N'[b] proportionally to each note's partial power
        let mut results = ArrayVec::<_, MAX_LOUDNESS_NOTES>::new();

        for (n_idx, &f0) in frequencies[..num_notes].iter().enumerate() {
            let mut note_sones = 0.0f32;

            for b in 0..NUM_BARK_BANDS {
                let note_power_in_band = self.scratch_note_band_power[n_idx][b];
                let total_power = self.band_powers[b].max(total_allocated_band_power[b]);
                if note_power_in_band > 0.0 && total_power > 0.0 {
                    let fraction = (note_power_in_band / total_power).min(1.0);
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

    /// Evaluates individual perceived loudness for detected `Notes` enums using harmonic peak apportioning.
    pub fn process_notes(
        &mut self,
        samples: &[f32],
        notes: &[Notes],
        max_harmonics: usize,
    ) -> ArrayVec<NoteLoudnessResult, MAX_LOUDNESS_NOTES> {
        let mut freqs = ArrayVec::<f32, MAX_LOUDNESS_NOTES>::new();
        for note in notes {
            if let Some(freq) = note.frequency() {
                let _ = freqs.try_push(freq);
            }
        }
        self.process_chord_notes(samples, freqs.as_slice(), max_harmonics)
    }
}

/// Converts perceived loudness in Sones into perceived Loudness Level in Phons.
///
/// * $L_N = 40 + 33.21928 \cdot \log_{10}(N)$ for $N \ge 1.0$ Sones.
/// * Compressive threshold mapping for $0 < N < 1.0$ Sones.
#[inline]
pub fn sones_to_phons(sones: f32) -> f32 {
    if sones >= 1.0 {
        40.0 + 33.21928 * sones.log10()
    } else if sones > 0.0 {
        40.0 * sones.powf(0.35)
    } else {
        0.0
    }
}

/// Maps a frequency in Hz to its corresponding Zwicker Bark critical band index (0..23).
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

/// Converts a frequency in Hz to the nearest MIDI note number (0..127).
#[inline]
pub fn hz_to_midi(freq_hz: f32) -> u8 {
    if freq_hz <= 0.0 || freq_hz.is_nan() {
        return 0;
    }
    let midi = 69.0 + 12.0 * (freq_hz / 440.0).log2();
    midi.round().clamp(0.0, 127.0) as u8
}
