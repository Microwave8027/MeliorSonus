# Audio Feature Extraction Specification: Instrument Target Matrix

This document defines the feature extraction pipeline beyond baseline static pitch transcription. While fixed-pitch percussion instruments (such as `Instrument::Piano`) primarily require discrete note events ($f_0$, onset velocity, duration, and median steady-state tuning offset), non-percussive and continuous-pitch instruments require extracting continuous expressive features from the Basic Pitch contour tensor ($T \times 264$) and complementary raw DSP pipelines.

---

## 1. Master Feature Registry

| Feature Key | Type / Unit | Extraction Source | Description |
| :--- | :--- | :--- | :--- |
| `tuning_offset_cents` | `f32` (cents, $[-50, +50]$) | Contour Argmax / Parabolic Interp | Median pitch deviation from equal temperament during steady-state sustain. |
| `pitch_jitter_cents` | `f32` (cents RMS) | Contour Residuals | Standard deviation of micro-pitch fluctuations around local trend line (unintentional instability). |
| `vibrato_rate_hz` | `f32` (Hz, $[0.0, 15.0]$) | Contour FFT / Zero-Crossing | Modulation frequency of cyclic pitch variation during sustain. |
| `vibrato_depth_cents` | `f32` (cents peak-to-peak) | Contour Hilbert / Peak Envelope | Total amplitude of cyclic pitch oscillation. |
| `vibrato_onset_delay_ms`| `u32` (milliseconds) | Contour Envelope Threshold | Latency between note onset transient and stabilization of regular vibrato. |
| `attack_scoop_cents` | `f32` (cents, $[-200, +200]$) | First $50\text{--}100\text{ ms}$ Contour Slope | Initial pitch trajectory offset prior to locking onto note center. |
| `attack_settle_time_ms` | `u32` (milliseconds) | Contour Gradient Convergence | Time required for the initial pitch transient to converge within $\pm 15\text{ cents}$ of steady-state. |
| `pitch_bend_extent_cents`| `f32` (cents, $[0, 1200+]$) | Continuous Contour Tracking | Maximal intentional monotonic pitch movement during a sustained note. |
| `portamento_slope` | `f32` ($\text{cents} / \text{sec}$) | Inter-Note Contour Segment | Linear or polynomial rate of continuous pitch transition between two note centers. |
| `release_fall_off_cents`| `f32` (cents) | Last $50\text{--}100\text{ ms}$ Contour Slope | Pitch drooping or upward flick during note termination. |
| `harmonic_clarity_hnr` | `f32` (dB) | Spectral DSP (Autocorrelation) | Harmonics-to-Noise Ratio measuring tone purity vs breathiness/bow friction. |
| `spectral_flux_attack` | `f32` (scalar) | Short-Time Fourier Transform | Rate of timbral transition at onset (distinguishes soft legato vs harsh articulation). |
---

## 2. Instrument-Specific Feature Matrix
| Instrument Variant | Vibrato (Rate/Depth) | Pitch Scoop / Drift | Pitch Bend / Slides | Jitter / Stability | Release Fall-Off | Harmonic Purity / HNR |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `Generic` | ● | ● | ● | ● | ● | ○ |
| `AcousticGuitar` | ○ | ● | ● | ● | - | ○ |
| `ElectricGuitar` | ● | ● | ● | ● | ○ | ○ |
| `ElectricBass4` | - | ● | ● | ● | - | ○ |
| `ElectricBass5` | - | ● | ● | ● | - | ○ |
| `Violin` | ● | ● | ● | ● | - | ● |
| `Viola` | ● | ● | ● | ● | - | ● |
| `Cello` | ● | ● | ● | ● | - | ● |
| `DoubleBass` | ● | ● | ● | ● | - | ● |
| `Flute` | ● | ● | - | ● | ○ | ● |
| `ClarinetBb` | ○ | ● | ● | ● | ○ | ● |
| `Oboe` | ● | ● | - | ● | ○ | ● |
| `Bassoon` | ● | ● | - | ● | ○ | ● |
| `AltoSax` | ● | ● | ● | ● | ● | ● |
| `TenorSax` | ● | ● | ● | ● | ● | ● |
| `TrumpetBb` | ● | ● | ● | ● | ● | ● |
| `FrenchHorn` | ○ | ● | - | ● | ○ | ● |
| `TromboneTenor` | ● | ● | ● | ● | ● | ● |
| `Tuba` | - | ● | - | ● | - | ● |
| `VoiceSoprano` | ● | ● | ● | ● | ● | ● |
| `VoiceTenor` | ● | ● | ● | ● | ● | ● |
| `VoiceBass` | ● | ● | ● | ● | ● | ● |
| `Custom { .. }` | Context | Context | Context | Context | Context | Context |

---

## 3. Instrument Specifications & Acoustic Rationales

### Fallback Variant

#### `Instrument::Generic`
* **Applicable Features:** Full feature suite (`tuning_offset_cents`, `vibrato_*`, `attack_scoop_cents`, `portamento_slope`, `pitch_jitter_cents`, `release_fall_off_cents`).
* **Acoustic Rationale:** Unconstrained polyphonic or monophonic input. Operates as an open-domain pipeline that does not constrain pitch bounds or modulation bands.
* **Evaluation Target:** Captures gross performance metrics (pitch error, general instability) when instrument identity is unclassified.

---

### Plucked & Fretted Strings

#### `Instrument::AcousticGuitar`
* **Applicable Features:** `pitch_bend_extent_cents`, `attack_scoop_cents`, `portamento_slope`, `pitch_jitter_cents`.
* **Acoustic Rationale:** Frets dictate discrete pitches, but string elasticity allows intentional upward pitch bending (typically $100\text{--}400\text{ cents}$) and fingerboard slides.
* **Performance Meaning:**
  * Detects fretting technique: hammer-ons and pull-offs show steep, instantaneous pitch transitions without new onset energy peaks.
  * Identifies excessive fret-hand pressure (accidental sharping of notes).

#### `Instrument::ElectricGuitar`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `pitch_bend_extent_cents`, `attack_scoop_cents`, `portamento_slope`, `pitch_jitter_cents`.
* **Acoustic Rationale:** Lower string tension and high-gain sustain enable wide manual finger vibrato (up to $\pm 100\text{ cents}$), whammy-bar dives, and sustained microtonal bends.
* **Performance Meaning:** Separates stylistic wide vibrato from fret buzz or intonation setup issues.

#### `Instrument::ElectricBass4` & `Instrument::ElectricBass5`
* **Applicable Features:** `attack_scoop_cents`, `portamento_slope`, `pitch_jitter_cents`, `tuning_offset_cents`.
* **Acoustic Rationale:** Thick core wire creates high inharmonicity and extended decay. Low frequency fundamental ($E_1 \approx 41.2\text{ Hz}$ on 4-string, $B_0 \approx 30.87\text{ Hz}$ on 5-string) requires careful temporal windowing. Vibrato is rare; glissando and fret slides dominate expressive movement.
* **Performance Meaning:** Measures timing precision and groove stability; evaluates whether fretboard slides land cleanly on the target beat and pitch center.

---

### Bowed Orchestral Strings

#### `Instrument::Violin` & `Instrument::Viola`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `vibrato_onset_delay_ms`, `attack_settle_time_ms`, `portamento_slope`, `pitch_jitter_cents`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Fretless continuous fingerboard. Pitch is entirely dependent on finger placement, and sound production relies on continuous bow friction (stick-slip phenomenon).
* **Performance Meaning:**
  * **Intonation Accuracy:** Quantifies whether finger placement aligns with pure vs tempered intervals.
  * **Vibrato Mechanics:** Evaluates maturity of wrist/arm vibrato (optimal classical standard: $5.5\text{--}7.0\text{ Hz}$ rate, $\pm 25\text{--}40\text{ cents}$ depth).
  * **Bow Noise (HNR):** Low HNR during sustain flags improper bow weight, speed, or contact point (sul ponticello / scratch).

#### `Instrument::Cello` & `Instrument::DoubleBass`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `attack_settle_time_ms`, `portamento_slope`, `pitch_jitter_cents`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Larger vibrating mass and longer string lengths lower the natural rate of vibrato ($4.5\text{--}6.0\text{ Hz}$ on cello; $3.5\text{--}5.0\text{ Hz}$ on double bass). Attack transients take longer to develop stable harmonic Helmholtz motion.
* **Performance Meaning:** Tracks the speed of pitch acquisition on fast low-register runs and distinguishes intentional orchestral shifting (portamento) from sluggish finger placement.

---

### Woodwinds

#### `Instrument::Flute`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `pitch_jitter_cents`, `harmonic_clarity_hnr`, `tuning_offset_cents`.
* **Acoustic Rationale:** Air-reed (embouchure hole jet) instrument. Pitch shifts sharply with air velocity and roll angle. Vibrato is produced via abdominal/diaphragmatic pulsation, creating coupled amplitude (tremolo) and frequency modulation.
* **Performance Meaning:** Jitter and HNR quantify breath stability and embouchure focus; tracks the tendency of the high register ($C_6\text{--}C_7$) to play excessively sharp during crescendos.

#### `Instrument::ClarinetBb`
* **Applicable Features:** `tuning_offset_cents`, `pitch_jitter_cents`, `attack_scoop_cents`, `portamento_slope`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Cylindrical bore behaving as a closed pipe (odd harmonics dominant). Classical performance mandates a straight tone (zero vibrato); jazz/klezmer performance relies heavily on wide reed scoops and lip glissandi.
* **Performance Meaning:**
  * Detects embouchure fatigue (pitch sagging flat during long sustained tones).
  * Evaluates smooth finger covering on register key crossings ($A_4 \to B_4$).

#### `Instrument::Oboe` & `Instrument::Bassoon`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `vibrato_onset_delay_ms`, `pitch_jitter_cents`, `attack_settle_time_ms`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Double reed with high backpressure. Highly sensitive to minute changes in lip pressure. Vibrato is tight and narrow ($5.0\text{--}7.0\text{ Hz}$, $\pm 15\text{--}30\text{ cents}$).
* **Performance Meaning:** Double reed instruments are prone to attack "cracking" or unstable micro-pitches on soft entrances; tracking `attack_settle_time_ms` reveals reed response and air-stream support.

#### `Instrument::AltoSax` & `Instrument::TenorSax`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `attack_scoop_cents`, `portamento_slope`, `release_fall_off_cents`, `pitch_jitter_cents`.
* **Acoustic Rationale:** Conical bore single-reed instrument with a highly flexible embouchure. Performers continuously modulate pitch via jaw movement, enabling vocal-like scoops, terminal drop-offs, and wide expressive vibrato.
* **Performance Meaning:** Distinguishes classical control (straight tone with terminal vibrato) from contemporary/jazz idioms (heavy lower-pitch scoops, bend entries, sub-tone air release).

---

### Brass

#### `Instrument::TrumpetBb`
* **Applicable Features:** `tuning_offset_cents`, `vibrato_rate_hz`, `vibrato_depth_cents`, `attack_scoop_cents`, `pitch_bend_extent_cents`, `pitch_jitter_cents`.
* **Acoustic Rationale:** Lip-reed coupled with valve combinations. Specific valve combinations are inherently out of tune (e.g., valves 1-2-3 or 1-3 are acoustically sharp). Lip bending is used to correct intonation or execute blues half-valve smears.
* **Performance Meaning:**
  * Assesses valve slide compensation on low $D_4$ and $C\sharp_4$.
  * Detects lip fatigue (instability, split notes during attacks).

#### `Instrument::FrenchHorn`
* **Applicable Features:** `tuning_offset_cents`, `pitch_jitter_cents`, `attack_settle_time_ms`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Extremely high harmonic order operation (players regularly play in harmonics 8 through 16 where pitch slots sit exceptionally close together). Vibrato is generally avoided in classical orchestral settings.
* **Performance Meaning:** Evaluates centering accuracy on attacks; prevents "motorboating" or split partials caused by incorrect right-hand horn placement or misaligned lip tension.

#### `Instrument::TromboneTenor`
* **Applicable Features:** `portamento_slope`, `pitch_bend_extent_cents`, `vibrato_rate_hz`, `vibrato_depth_cents`, `attack_scoop_cents`, `pitch_jitter_cents`.
* **Acoustic Rationale:** Continuous telescoping slide allows true physical glissandi across multiple positions without valve steps.
* **Performance Meaning:**
  * **Slide Accuracy:** Measures how accurately the slide lands on target positions without hunting/correcting after the attack.
  * **Legato vs Portamento:** Differentiates clean lip-slurs from unintentional slide smearing between notes.

#### `Instrument::Tuba`
* **Applicable Features:** `tuning_offset_cents`, `pitch_jitter_cents`, `attack_settle_time_ms`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Large conical brass bore operating in the sub-bass and bass frequency registers. Transient build-up takes up to $80\text{ ms}$. Vibrato is non-standard.
* **Performance Meaning:** Focuses on attack promptness, core fundamental resonance, and breath control stability.

---

### Vocals

#### `Instrument::VoiceSoprano`, `Instrument::VoiceTenor`, `Instrument::VoiceBass`
* **Applicable Features:** `vibrato_rate_hz`, `vibrato_depth_cents`, `vibrato_onset_delay_ms`, `attack_scoop_cents`, `portamento_slope`, `release_fall_off_cents`, `pitch_jitter_cents`, `harmonic_clarity_hnr`.
* **Acoustic Rationale:** Biomechanical vocal fold oscillation with dynamic vocal tract filtering. Pitch is infinitely continuous and susceptible to physiological stress, breath support variations, and vowel formant shifts.
* **Performance Meaning:**
  * **Vibrato Integrity:** Detects vocal wobbles (vibrato $< 4.5\text{ Hz}$), bleats (vibrato $> 7.5\text{ Hz}$), or straight-tone singing with intentional delayed vibrato.
  * **Attack Trajectory:** Differentiates confident onsets (glottal/balanced attacks) from flat scooping.
  * **Vocal Health:** Elevated pitch jitter combined with depressed HNR points directly to hoarseness, vocal strain, or breath leaks.

---

### Custom Configuration

#### `Instrument::Custom { hpf_cutoff_hz, min_f0_hz, max_f0_hz, harmonic_ceiling_hz }`
* **Applicable Features:** Dynamically determined based on the user-defined frequency constraints.
* **Acoustic Rationale:** Allows defining specialized acoustic instruments (e.g., Erhu, Shakuhachi, Bagpipes, Bass Clarinet) by setting precise DSP bandpass filtering and fundamental search bounds.
* **Feature Extraction Pipeline:**
  1. Clamps Basic Pitch contour analysis between `min_f0_hz` and `max_f0_hz`.
  2. Applies a high-pass filter at `hpf_cutoff_hz` to eliminate sub-audio rumble or mechanical handling noise.
  3. Uses `harmonic_ceiling_hz` to bound HNR and spectral flux calculations, preventing high-frequency noise from corrupting timbre evaluations.
