pub struct Note {
    pitch: Pitch,
    octave: Octave,
    tonality_offset: i8, // meant to check for cent sharpness
}

pub enum Pitch {
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

#[allow(non_camel_case_types)]
pub enum Octave {
    OutOfRange,
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

impl Note {
    pub fn get_note(sampling_rate: u32, tau: f32) -> Option<Note> {
        if tau == 0.0 {
            return None;
        }

        let frequency = sampling_rate as f32 / tau;
        let n = 69.0 + 12.0 * ((frequency / 440.0).log2());
        let octave = Octave::midi_to_note(n.round() as u8);
        let pitch = Pitch::get_pitch(n.round() as u8);
        let tonality_offset = ((n - n.round()) * 100.0).round() as i8;
        Some(Note {
            pitch,
            octave,
            tonality_offset,
        })
    }
}

impl Octave {
    fn midi_to_note(n: u8) -> Octave {
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
            _ => Octave::OutOfRange, // technically should be filtered out
        }
    }
}

impl Pitch {
    fn get_pitch(n: u8) -> Pitch {
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
            _ => Pitch::C, // never reachable, doesnt matter
        }
    }
}
