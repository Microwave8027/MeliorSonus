use super::instrument::{Instrument, InstrumentAcousticProfile};
use crate::audio_processing::processing::functions::articulation::articulation_classifier::{
    classify_articulation, classify_damping, detect_mashed_key,
};

/// Dynamic classification level based on acoustic decibels relative to full scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicLevel {
    Pianississimo, // ppp (< -45 dBFS)
    Pianissimo,    // pp  (-45 to -38 dBFS)
    Piano,         // p   (-38 to -30 dBFS)
    MezzoPiano,    // mp  (-30 to -24 dBFS)
    MezzoForte,    // mf  (-24 to -18 dBFS)
    Forte,         // f   (-18 to -12 dBFS)
    Fortissimo,    // ff  (-12 to -6 dBFS)
    Fortississimo, // fff (> -6 dBFS)
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
/// * `rise_duration: f32` — Duration in seconds from onset strike to peak loudness (attack phase). Range: `0.005` to `2.0+` seconds. IGNORE FOR PIANO
/// * `attack_slope: f32` — Initial attack velocity steepness in dBFS per second. Range: `0.0` to `1000.0+` dBFS/s.
/// * `note_duration: f32` — Total sounded note duration in seconds from initial strike to release. Range: `> 0.0` seconds.
/// * `note_striked: u128` — Timestamp in epoch milliseconds when note onset originally occurred. Range: `>= 0` ms.
/// * `articulation: NoteArticulation` — Pedagogical articulation classification (Normal, Staccato, Tenuto, Legato, Marcato). Range: `NoteArticulation` enum variants.
/// * `is_mashed: bool` — Flag indicating whether the key was struck excessively hard with low-end thump impact. Range: `true` / `false`.\n/// * `is_flat: bool` — Flag indicating whether the sustained pitch average was below the instrument's flat intonation threshold. Range: `true` / `false`.
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

    pub fn note_striked(&self) -> u128 {
        match self {
            Notes::Start(s) => s.note_striked,
            Notes::End(e) => e.note_striked,
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
}

impl StartNote {
    pub fn update_note_striked(&mut self, timestamp: u128) {
        self.note_striked = timestamp;
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
        }
    }

    pub fn to_start_note(&self) -> StartNote {
        let dynamic = DynamicLevel::from_dbfs(self.onset_dbfs);
        let velocity = calculate_midi_velocity(self.onset_dbfs, self.initial_crest_factor);
        StartNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.onset_dbfs,
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

    pub fn accumulate_frame(&mut self, tonality_offset: i8, centroid: f32) {
        self.sum_cents += tonality_offset as i32;
        self.min_cents = self.min_cents.min(tonality_offset);
        self.max_cents = self.max_cents.max(tonality_offset);
        self.sum_spectral_centroid += centroid;
        self.active_frames += 1;
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
        let dynamic = DynamicLevel::from_dbfs(self.peak_dbfs);
        let damping = classify_damping(note_duration, profile);

        EndNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            avg_cents_offset: avg_cents,
            loudness_dbfs: self.peak_dbfs,
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
