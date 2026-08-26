use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct StartNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8, // cent sharpness/flatness offset
    pub loudness_dbfs: f32,
    pub note_striked: u128,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub loudness_dbfs: f32,
    pub rise_duration: f32,
    pub note_duration: f32,
    pub note_striked: u128,
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

#[derive(Clone, Debug, PartialEq)]
pub struct RecordNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub tonality_offset: i8,
    pub peak_dbfs: f32,
    pub last_dbfs: f32,
    pub note_striked: u128,
    pub last_timestamp: u128,
    pub rise_duration: Option<f32>,
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
            note_striked,
            last_timestamp: note_striked,
            rise_duration: None,
        }
    }

    pub fn to_start_note(&self) -> StartNote {
        StartNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.peak_dbfs,
            note_striked: self.note_striked,
        }
    }

    pub fn record_rise_time(&mut self, timestamp: u128) {
        if self.rise_duration.is_none() {
            let rise_ms = timestamp.saturating_sub(self.note_striked);
            self.rise_duration = Some(rise_ms as f32 / 1000.0);
        }
        self.last_timestamp = timestamp;
    }

    pub fn total_active_duration(&self) -> Duration {
        Duration::from_millis(self.last_timestamp.saturating_sub(self.note_striked) as u64)
    }

    pub fn add_peak_dbfs(&mut self, dbfs: f32) {
        if dbfs > self.peak_dbfs {
            self.peak_dbfs = dbfs;
        }
    }

    pub fn set_last_dbfs(&mut self, dbfs: f32) {
        self.last_dbfs = dbfs;
    }

    pub fn into_end_note(self) -> EndNote {
        let total_secs = (self.last_timestamp.saturating_sub(self.note_striked)) as f32 / 1000.0;
        let rise_duration = self.rise_duration.unwrap_or(total_secs);
        EndNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.peak_dbfs,
            rise_duration,
            note_duration: total_secs,
            note_striked: self.note_striked,
        }
    }

    pub fn into_end_note_with_timestamp(self, final_timestamp: u128) -> EndNote {
        let total_secs = (final_timestamp.saturating_sub(self.note_striked)) as f32 / 1000.0;
        let rise_duration = self.rise_duration.unwrap_or(total_secs);
        EndNote {
            pitch: self.pitch,
            octave: self.octave,
            tonality_offset: self.tonality_offset,
            loudness_dbfs: self.peak_dbfs,
            rise_duration,
            note_duration: total_secs,
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
