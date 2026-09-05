use super::instrument::{Instrument, InstrumentAcousticProfile};
use crate::audio_processing::processing::functions::articulation::articulation_classifier::{
    classify_articulation, classify_damping, detect_mashed_key,
};
use crate::audio_processing::processing::functions::spectral::sones_to_phons;

/// Dynamic classification level based on acoustic decibels relative to full scale or perceived loudness level in Phons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicLevel {
    Pianississimo, // ppp (< -45 dBFS / < 34 Phons)
    Pianissimo,    // pp  (-45 to -38 dBFS / 34 to 42 Phons)
    Piano,         // p   (-38 to -30 dBFS / 42 to 50 Phons)
    MezzoPiano,    // mp  (-30 to -24 dBFS / 50 to 58 Phons)
    MezzoForte,    // mf  (-24 to -18 dBFS / 58 to 66 Phons)
    Forte,         // f   (-18 to -12 dBFS / 66 to 74 Phons)
    Fortissimo,    // ff  (-12 to -6 dBFS / 74 to 82 Phons)
    Fortississimo, // fff (> -6 dBFS / > 82 Phons)
}

impl DynamicLevel {
    pub fn from_dbfs(dbfs: f32) -> Self {
        if dbfs < -45.0 {
            DynamicLevel::Pianississimo
        } else if dbfs < -38.0 {
            DynamicLevel::Pianissimo
        } else if dbfs < -30.0 {
            DynamicLevel::Piano
        } else if dbfs < -24.0 {
            DynamicLevel::MezzoPiano
        } else if dbfs < -18.0 {
            DynamicLevel::MezzoForte
        } else if dbfs < -12.0 {
            DynamicLevel::Forte
        } else if dbfs < -6.0 {
            DynamicLevel::Fortissimo
        } else {
            DynamicLevel::Fortississimo
        }
    }

    pub fn from_phons(phons: f32) -> Self {
        if phons.is_nan() || phons < 34.0 {
            DynamicLevel::Pianississimo
        } else if phons < 42.0 {
            DynamicLevel::Pianissimo
        } else if phons < 50.0 {
            DynamicLevel::Piano
        } else if phons < 58.0 {
            DynamicLevel::MezzoPiano
        } else if phons < 66.0 {
            DynamicLevel::MezzoForte
        } else if phons < 74.0 {
            DynamicLevel::Forte
        } else if phons < 82.0 {
            DynamicLevel::Fortissimo
        } else {
            DynamicLevel::Fortississimo
        }
    }
}

/// Damping profile upon note release (staccato key release vs. pedaled sustain).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DampingProfile {
    DryDamped,      // Sharp, rapid energy cutoff (< 60ms)
    PedalSustained, // Exponential long ring-out / damper lifted (> 1.5s)
    HalfPedal,      // Moderate decay
}

/// Computes calibrated 1..127 MIDI touch velocity from attack dBFS and transient crest factor.
pub fn calculate_midi_velocity(loudness_dbfs: f32, crest_factor: f32) -> u8 {
    let min_db = -55.0f32;
    let max_db = -5.0f32;
    let norm_db = ((loudness_dbfs - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
    let crest_bonus = ((crest_factor - 6.0) / 18.0).clamp(0.0, 0.2);
    let vel = ((norm_db * 0.85 + crest_bonus) * 127.0).round() as u8;
    vel.clamp(1, 127)
}

/// Represents the onset strike of a musical note in the real-time DSP/Neural pipeline.
///
/// Emitted as soon as the note attack envelope transitions from `Idle` to `Rise`.
///
/// # Fields & Values:
/// * `pitch: Pitch` — Musical pitch class (e.g. C, CsDf, D... B, or None). Range: `Pitch` enum variants.
/// * `octave: Octave` — Scientific pitch notation octave register (O_1 to O10, or OutOfRange). Range: `Octave` enum variants.
/// * `tonality_offset: i8` — Initial intonation deviation in cents relative to equal temperament (A4 = 440 Hz). Range: `-50` to `+50` cents.
/// * `loudness_dbfs: f32` — Attack/onset root-mean-square loudness in decibels relative to full scale. Range: `-120.0` to `0.0` dBFS.
/// * Perceived linear attack loudness in Sones (None for monophonic MPM path)
/// * Perceived linear attack loudness in Phons (None for monophonic MPM path)
/// * `note_striked: u128` — Timestamp in epoch milliseconds when the initial note onset occurred. Range: `>= 0` ms.
/// * `crest_factor: f32` — Ratio of peak amplitude to RMS energy in dB at attack (transient percussiveness). Range: `0.0` to `30.0+` dB.
/// * `sub_thump_dbfs: f32` — Low-frequency energy (20–80 Hz) at strike time for keybed collision detection. Range: `-120.0` to `0.0` dBFS.
/// * `mpm_clarity: Option<f32>` — McLeod Pitch Method normalized square difference (NSDF) periodicity confidence at onset. Range: `Some(0.0..=1.0)` (monophonic path) or `None` (polyphonic / uncalculated).
/// * `velocity: u8` — Calibrated MIDI strike velocity. Range: `1` to `127`.
/// * `dynamic: DynamicLevel` — Standard musical dynamic marking (ppp to fff).
#[derive(Clone, Debug, PartialEq)]
pub struct StartNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub loudness_dbfs: f32,
    pub sones: Option<f32>,
    pub phons: Option<f32>,
    pub note_striked: u128,
    pub crest_factor: f32,
    pub sub_thump_dbfs: f32,
    pub mpm_clarity: Option<f32>,
    pub velocity: u8,
    pub dynamic: DynamicLevel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteArticulation {
    Normal,
    Staccato,
    Tenuto,
    Legato,
    Marcato,
}

/// Represents a finalized, fully evaluated musical note emitted upon release or restrike.
///
/// Contains pedagogical performance metrics including sustained pitch stability,
/// articulation classifications, attack velocity slope, keybed impact, and spectral timbre.
///
/// # Fields & Values:
/// * `pitch: Pitch` — Musical pitch class of the performed note. Range: `Pitch` enum variants.
/// * `octave: Octave` — Musical octave register of the performed note. Range: `Octave` enum variants.
/// * `tonality_offset: i8` — Initial attack intonation deviation in cents. Range: `-50` to `+50` cents.
/// * `avg_cents_offset: i8` — Average intonation deviation in cents across all sustained active frames. Range: `-50` to `+50` cents.
/// * `loudness_dbfs: f32` — Peak sustained volume/energy reached during the note envelope. Range: `-120.0` to `0.0` dBFS.
/// * `peak_sones: Option<f32>` — Peak sustained perceived loudness in Sones (None for MPM path).
/// * `peak_phons: Option<f32>` — Peak sustained perceived loudness level in Phons (None for MPM path).
/// * `avg_sones: Option<f32>` — Average sustained perceived loudness in Sones (None for MPM path).
/// * `avg_phons: Option<f32>` — Average sustained perceived loudness level in Phons (None for MPM path).
/// * `rise_duration: f32` — Duration in seconds from onset strike to peak loudness (attack phase). Range: `0.005` to `2.0+` seconds. IGNORE FOR PIANO
/// * `attack_slope: f32` — Initial attack velocity steepness in dBFS per second. Range: `0.0` to `1000.0+` dBFS/s.
/// * `note_duration: f32` — Total sounded note duration in seconds from initial strike to release. Range: `> 0.0` seconds.
/// * `note_striked: u128` — Timestamp in epoch milliseconds when note onset originally occurred. Range: `>= 0` ms.
/// * `articulation: NoteArticulation` — Pedagogical articulation classification (Normal, Staccato, Tenuto, Legato, Marcato). Range: `NoteArticulation` enum variants.
/// * `is_mashed: bool` — Flag indicating whether the key was struck excessively hard with low-end thump impact. Range: `true` / `false`.
/// * `is_flat: bool` — Flag indicating whether the sustained pitch average was below the instrument's flat intonation threshold. Range: `true` / `false`.
/// * `spectral_centroid: f32` — Average spectral center-of-mass in Hertz (spectral brightness / timbral richness). Range: `0.0` to `Nyquist (SampleRate / 2)` Hz.
/// * `mpm_clarity: Option<f32>` — McLeod Pitch Method normalized square difference (NSDF) periodicity confidence. Range: `Some(0.0..=1.0)` or `None`.
/// * `velocity: u8` — Calibrated MIDI strike velocity. Range: `1` to `127`.
/// * `dynamic: DynamicLevel` — Standard musical dynamic marking (ppp to fff).
/// * `damping: DampingProfile` — Damping state upon release (DryDamped, PedalSustained, HalfPedal).
#[derive(Clone, Debug, PartialEq)]
pub struct EndNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub avg_cents_offset: i8,
    pub loudness_dbfs: f32,
    pub peak_sones: Option<f32>,
    pub peak_phons: Option<f32>,
    pub avg_sones: Option<f32>,
    pub avg_phons: Option<f32>,
    pub rise_duration: f32,
    pub attack_slope: f32,
    pub note_duration: f32,
    pub note_striked: u128,
    pub articulation: NoteArticulation,
    pub is_mashed: bool,
    pub is_flat: bool,
    pub spectral_centroid: f32,
    pub mpm_clarity: Option<f32>,
    pub velocity: u8,
    pub dynamic: DynamicLevel,
    pub damping: DampingProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Notes {
    Start(StartNote),
    End(EndNote),
}

impl Notes {
    pub fn is_start(&self) -> bool {
        matches!(self, Notes::Start(_))
    }

    pub fn is_end(&self) -> bool {
        matches!(self, Notes::End(_))
    }

    pub fn start(&self) -> Option<&StartNote> {
        match self {
            Notes::Start(s) => Some(s),
            _ => None,
        }
    }

    pub fn end(&self) -> Option<&EndNote> {
        match self {
            Notes::End(e) => Some(e),
            _ => None,
        }
    }

    pub fn pitch(&self) -> Pitch {
        match self {
            Notes::Start(s) => s.pitch,
            Notes::End(e) => e.pitch,
        }
    }

    pub fn octave(&self) -> Octave {
        match self {
            Notes::Start(s) => s.octave,
            Notes::End(e) => e.octave,
        }
    }

    pub fn tonality_offset(&self) -> i8 {
        match self {
            Notes::Start(s) => s.tonality_offset,
            Notes::End(e) => e.avg_cents_offset,
        }
    }

    pub fn note_striked(&self) -> u128 {
        match self {
            Notes::Start(s) => s.note_striked,
            Notes::End(e) => e.note_striked,
        }
    }

    /// Computes the fundamental frequency in Hz for this note event,
    /// factoring in pitch class, octave register, and microtonal cent deviation.
    pub fn frequency(&self) -> Option<f32> {
        let (pitch, octave, cents) = match self {
            Notes::Start(s) => (s.pitch, s.octave, s.tonality_offset),
            Notes::End(e) => (e.pitch, e.octave, e.avg_cents_offset),
        };
        note_to_frequency(pitch, octave, cents)
    }

    /// Perceived linear loudness in Sones (attack loudness for Start, avg loudness for End).
    pub fn loudness_sones(&self) -> Option<f32> {
        match self {
            Notes::Start(s) => s.sones,
            Notes::End(e) => e.avg_sones,
        }
    }

    /// Perceived loudness level in Phons (attack loudness level for Start, avg level for End).
    pub fn loudness_phons(&self) -> Option<f32> {
        match self {
            Notes::Start(s) => s.phons,
            Notes::End(e) => e.avg_phons,
        }
    }

    /// Peak perceived linear loudness in Sones.
    pub fn peak_sones(&self) -> Option<f32> {
        match self {
            Notes::Start(s) => s.sones,
            Notes::End(e) => e.peak_sones,
        }
    }

    /// Peak perceived loudness level in Phons.
    pub fn peak_phons(&self) -> Option<f32> {
        match self {
            Notes::Start(s) => s.phons,
            Notes::End(e) => e.peak_phons,
        }
    }
}

impl From<StartNote> for Notes {
    fn from(n: StartNote) -> Self {
        Notes::Start(n)
    }
}

impl From<EndNote> for Notes {
    fn from(n: EndNote) -> Self {
        Notes::End(n)
    }
}

impl EndNote {
    pub fn update_note_striked(&mut self, timestamp: u128) {
        self.note_striked = timestamp;
    }

    /// Computes the fundamental frequency in Hz for this end note.
    pub fn frequency(&self) -> Option<f32> {
        note_to_frequency(self.pitch, self.octave, self.avg_cents_offset)
    }

    pub fn peak_sones(&self) -> Option<f32> {
        self.peak_sones
    }

    pub fn peak_phons(&self) -> Option<f32> {
        self.peak_phons
    }

    pub fn avg_sones(&self) -> Option<f32> {
        self.avg_sones
    }

    pub fn avg_phons(&self) -> Option<f32> {
        self.avg_phons
    }
}

impl StartNote {
    pub fn update_note_striked(&mut self, timestamp: u128) {
        self.note_striked = timestamp;
    }

    /// Computes the fundamental frequency in Hz for this start note.
    pub fn frequency(&self) -> Option<f32> {
        note_to_frequency(self.pitch, self.octave, self.tonality_offset)
    }

    pub fn loudness_sones(&self) -> Option<f32> {
        self.sones
    }

    pub fn loudness_phons(&self) -> Option<f32> {
        self.phons
    }
}

pub fn update_note_striked(note: &mut EndNote, timestamp: u128) {
    note.update_note_striked(timestamp);
}

#[derive(Clone, Debug)]
pub struct RecordNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub onset_dbfs: f32,
    pub peak_dbfs: f32,
    pub last_dbfs: f32,
    pub note_striked: u128,
    pub last_timestamp: u128,
    pub rise_duration: Option<f32>,
    pub initial_crest_factor: f32,
    pub initial_sub_thump_dbfs: f32,
    pub sum_cents: i32,
    pub min_cents: i8,
    pub max_cents: i8,
    pub sum_spectral_centroid: f32,
    pub active_frames: usize,
    pub mpm_clarity: Option<f32>,
    pub onset_sones: Option<f32>,
    pub onset_phons: Option<f32>,
    pub peak_sones: Option<f32>,
    pub sum_sones: Option<f32>,
}

impl RecordNote {
    pub fn new(
        pitch: Pitch,
        octave: Octave,
        tonality_offset: i8,
        peak_dbfs: f32,
        note_striked: u128,
        mpm_clarity: Option<f32>,
    ) -> Self {
        Self {
            pitch,
            octave,
            tonality_offset,
            onset_dbfs: peak_dbfs,
            peak_dbfs,
            last_dbfs: peak_dbfs,
            note_striked,
            last_timestamp: note_striked,
            rise_duration: None,
            initial_crest_factor: 12.0,
            initial_sub_thump_dbfs: -60.0,
            sum_cents: tonality_offset as i32,
            min_cents: tonality_offset,
            max_cents: tonality_offset,
            sum_spectral_centroid: 800.0,
            active_frames: 1,
            mpm_clarity,
            onset_sones: None,
            onset_phons: None,
            peak_sones: None,
            sum_sones: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_features(
        pitch: Pitch,
        octave: Octave,
        tonality_offset: i8,
        peak_dbfs: f32,
        note_striked: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        mpm_clarity: Option<f32>,
        onset_sones: Option<f32>,
        onset_phons: Option<f32>,
    ) -> Self {
        Self {
            pitch,
            octave,
            tonality_offset,
            onset_dbfs: peak_dbfs,
            peak_dbfs,
            last_dbfs: peak_dbfs,
            note_striked,
            last_timestamp: note_striked,
            rise_duration: None,
            initial_crest_factor: crest_factor,
            initial_sub_thump_dbfs: sub_thump_dbfs,
            sum_cents: tonality_offset as i32,
            min_cents: tonality_offset,
            max_cents: tonality_offset,
            sum_spectral_centroid: spectral_centroid,
            active_frames: 1,
            mpm_clarity,
            onset_sones,
            onset_phons,
            peak_sones: onset_sones,
            sum_sones: onset_sones,
        }
    }

    pub fn to_start_note(&self) -> StartNote {
        let dynamic = self
            .onset_phons
            .map(DynamicLevel::from_phons)
            .unwrap_or_else(|| DynamicLevel::from_dbfs(self.onset_dbfs));
        let velocity = calculate_midi_velocity(self.onset_dbfs, self.initial_crest_factor);
        StartNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.onset_dbfs,
            sones: self.onset_sones,
            phons: self.onset_phons,
            note_striked: self.note_striked,
            crest_factor: self.initial_crest_factor,
            sub_thump_dbfs: self.initial_sub_thump_dbfs,
            mpm_clarity: self.mpm_clarity,
            velocity,
            dynamic,
        }
    }

    pub fn add_peak_dbfs(&mut self, dbfs: f32) {
        if dbfs > self.peak_dbfs {
            self.peak_dbfs = dbfs;
        }
        self.last_dbfs = dbfs;
    }

    pub fn record_rise_time(&mut self, timestamp: u128) {
        if self.rise_duration.is_none() {
            let diff = timestamp.saturating_sub(self.note_striked) as f32 / 1000.0;
            self.rise_duration = Some(diff.max(0.005));
        }
    }

    pub fn accumulate_frame(&mut self, tonality_offset: i8, centroid: f32, sones: Option<f32>) {
        self.sum_cents += tonality_offset as i32;
        self.min_cents = self.min_cents.min(tonality_offset);
        self.max_cents = self.max_cents.max(tonality_offset);
        self.sum_spectral_centroid += centroid;
        self.active_frames += 1;

        if let Some(s) = sones {
            self.sum_sones = Some(self.sum_sones.unwrap_or(0.0) + s);
            if self.peak_sones.map_or(true, |p| s > p) {
                self.peak_sones = Some(s);
            }
        }
    }

    pub fn onset_sones(&self) -> Option<f32> {
        self.onset_sones
    }

    pub fn onset_phons(&self) -> Option<f32> {
        self.onset_phons
    }

    pub fn peak_sones(&self) -> Option<f32> {
        self.peak_sones
    }

    pub fn avg_sones(&self) -> Option<f32> {
        self.sum_sones.map(|sum| {
            if self.active_frames > 0 {
                sum / (self.active_frames as f32)
            } else {
                sum
            }
        })
    }

    pub fn into_end_note(self) -> EndNote {
        let profile = Instrument::Generic.acoustic_profile();
        let last_ts = self.last_timestamp;
        self.into_end_note_with_profile(last_ts, false, &profile)
    }

    pub fn into_end_note_with_profile(
        self,
        current_timestamp: u128,
        is_legato: bool,
        profile: &InstrumentAcousticProfile,
    ) -> EndNote {
        let total_ms = current_timestamp.saturating_sub(self.note_striked);
        let note_duration = (total_ms as f32) / 1000.0;
        let effective_rise = self.rise_duration.unwrap_or(0.005).max(0.005);
        let attack_slope = (self.peak_dbfs - self.onset_dbfs).abs() / effective_rise;

        let avg_cents = if self.active_frames > 0 {
            (self.sum_cents as f32 / self.active_frames as f32).round() as i8
        } else {
            self.tonality_offset
        };

        let avg_centroid = if self.active_frames > 0 {
            self.sum_spectral_centroid / self.active_frames as f32
        } else {
            800.0
        };

        let velocity = calculate_midi_velocity(self.peak_dbfs, self.initial_crest_factor);

        let articulation = classify_articulation(
            note_duration,
            is_legato,
            attack_slope,
            effective_rise,
            self.initial_crest_factor,
            velocity,
            profile,
        );

        let is_mashed = detect_mashed_key(
            self.initial_crest_factor,
            self.initial_sub_thump_dbfs,
            profile,
        );

        let is_flat = avg_cents <= profile.flat_cents_threshold;
        let damping = classify_damping(note_duration, profile);

        let avg_sones = self.avg_sones();
        let avg_phons = avg_sones.map(sones_to_phons);
        let peak_sones = self.peak_sones;
        let peak_phons = self.peak_sones.map(sones_to_phons);
        let dynamic = peak_phons
            .map(DynamicLevel::from_phons)
            .unwrap_or_else(|| DynamicLevel::from_dbfs(self.peak_dbfs));

        EndNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            avg_cents_offset: avg_cents,
            loudness_dbfs: self.peak_dbfs,
            peak_sones,
            peak_phons,
            avg_sones,
            avg_phons,
            rise_duration: effective_rise,
            attack_slope,
            note_duration,
            note_striked: self.note_striked,
            articulation,
            is_mashed,
            is_flat,
            spectral_centroid: avg_centroid,
            mpm_clarity: self.mpm_clarity,
            velocity,
            dynamic,
            damping,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Pitch {
    None,
    C,
    CsDf,
    D,
    DsEf,
    E,
    F,
    FsGf,
    G,
    GsAf,
    A,
    AsBf,
    B,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Octave {
    OutOfRange,
    #[allow(non_camel_case_types)]
    O_1,
    O0,
    O1,
    O2,
    O3,
    O4,
    O5,
    O6,
    O7,
    O8,
    O9,
    O10,
}

pub fn get_note(frequency: f32) -> Option<(Pitch, Octave, i8)> {
    if frequency <= 0.0 || frequency.is_nan() || frequency.is_infinite() {
        return None;
    }

    let n = 69.0 + 12.0 * ((frequency / 440.0).log2());
    if !(0.0..=127.0).contains(&n) {
        return None;
    }

    let midi = n.round() as u8;
    let octave = Octave::midi_to_note(midi);
    let pitch = Pitch::get_pitch(midi);

    if octave == Octave::OutOfRange || pitch == Pitch::None {
        return None;
    }

    let tonality_offset = ((n - n.round()) * 100.0).round() as i8;
    Some((pitch, octave, tonality_offset))
}

pub fn midi_to_freq(midi_note: u8) -> f32 {
    440.0 * 2.0f32.powf((midi_note as f32 - 69.0) / 12.0)
}

impl Octave {
    pub fn midi_to_note(n: u8) -> Octave {
        match n / 12 {
            0 => Octave::O_1,
            1 => Octave::O0,
            2 => Octave::O1,
            3 => Octave::O2,
            4 => Octave::O3,
            5 => Octave::O4,
            6 => Octave::O5,
            7 => Octave::O6,
            8 => Octave::O7,
            9 => Octave::O8,
            10 => Octave::O9,
            11 => Octave::O10,
            _ => Octave::OutOfRange,
        }
    }
}

impl Pitch {
    pub fn get_pitch(n: u8) -> Pitch {
        match n % 12 {
            0 => Pitch::C,
            1 => Pitch::CsDf,
            2 => Pitch::D,
            3 => Pitch::DsEf,
            4 => Pitch::E,
            5 => Pitch::F,
            6 => Pitch::FsGf,
            7 => Pitch::G,
            8 => Pitch::GsAf,
            9 => Pitch::A,
            10 => Pitch::AsBf,
            11 => Pitch::B,
            _ => Pitch::None,
        }
    }
}

/// Converts a (Pitch, Octave) pair into a standard MIDI note number (0..=127).
pub fn pitch_octave_to_midi(pitch: Pitch, octave: Octave) -> Option<u8> {
    let pitch_class = match pitch {
        Pitch::C => 0,
        Pitch::CsDf => 1,
        Pitch::D => 2,
        Pitch::DsEf => 3,
        Pitch::E => 4,
        Pitch::F => 5,
        Pitch::FsGf => 6,
        Pitch::G => 7,
        Pitch::GsAf => 8,
        Pitch::A => 9,
        Pitch::AsBf => 10,
        Pitch::B => 11,
        Pitch::None => return None,
    };

    let octave_num: i8 = match octave {
        Octave::O_1 => -1,
        Octave::O0 => 0,
        Octave::O1 => 1,
        Octave::O2 => 2,
        Octave::O3 => 3,
        Octave::O4 => 4,
        Octave::O5 => 5,
        Octave::O6 => 6,
        Octave::O7 => 7,
        Octave::O8 => 8,
        Octave::O9 => 9,
        Octave::O10 | Octave::OutOfRange => return None,
    };

    let midi = (octave_num + 1) * 12 + pitch_class;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

/// Converts a (Pitch, Octave, tonality_offset_cents) triplet into an exact frequency in Hz.
pub fn note_to_frequency(pitch: Pitch, octave: Octave, tonality_offset_cents: i8) -> Option<f32> {
    let midi = pitch_octave_to_midi(pitch, octave)?;
    let base_freq = midi_to_freq(midi);
    if tonality_offset_cents == 0 {
        Some(base_freq)
    } else {
        Some(base_freq * 2.0f32.powf(tonality_offset_cents as f32 / 1200.0))
    }
}
