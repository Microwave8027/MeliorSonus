use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8, // meant to check for cent sharpness
    pub loudness_dbfs: f32,
    pub rise_duration: f32,
    pub note_duration: f32,
    pub note_striked: u128,
}

impl Note {
    pub fn update_note_striked(&mut self, timestamp: u128) {
        self.note_striked = timestamp;
    }
}

pub fn update_note_striked(note: &mut Note, timestamp: u128) {
    note.update_note_striked(timestamp);
}

#[derive(Clone, Debug)]
pub struct RecordNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub peak_dbfs: f32,
    pub last_dbfs: f32,
    pub beginning: Instant,
    pub rise_time: Option<Duration>,
    pub note_striked: u128,
}

impl RecordNote {
    pub fn new(
        pitch: Pitch,
        octave: Octave,
        tonality_offset: i8,
        peak_dbfs: f32,
        note_striked: u128,
    ) -> Self {
        Self {
            pitch,
            octave,
            tonality_offset,
            peak_dbfs,
            last_dbfs: peak_dbfs,
            beginning: Instant::now(),
            rise_time: None,
            note_striked,
        }
    }

    pub fn record_rise_time(&mut self) {
        if self.rise_time.is_none() {
            self.rise_time = Some(self.beginning.elapsed());
        }
    }

    pub fn total_active_duration(&self) -> Duration {
        self.beginning.elapsed()
    }

    pub fn add_peak_dbfs(&mut self, dbfs: f32) {
        if dbfs > self.peak_dbfs {
            self.peak_dbfs = dbfs;
        }
    }

    pub fn set_last_dbfs(&mut self, dbfs: f32) {
        self.last_dbfs = dbfs;
    }

    pub fn into_note(self) -> Note {
        let total_duration = self.total_active_duration();
        let rise_duration = self.rise_time.unwrap_or(total_duration);
        Note {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.peak_dbfs,
            rise_duration: rise_duration.as_secs_f32(),
            note_duration: total_duration.as_secs_f32(),
            note_striked: self.note_striked,
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
