#![allow(non_camel_case_types)]

use rkyv::{Archive, Deserialize, Serialize};

/// Octave registers matching scientific pitch notation.
#[allow(non_camel_case_types)]
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Octave {
    pub fn from_midi(midi: u8) -> Self {
        match midi / 12 {
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

    pub fn to_audio_octave(&self) -> crate::audio_processing::instruments::notes::Octave {
        match self {
            Octave::OutOfRange => crate::audio_processing::instruments::notes::Octave::OutOfRange,
            Octave::O_1 => crate::audio_processing::instruments::notes::Octave::O_1,
            Octave::O0 => crate::audio_processing::instruments::notes::Octave::O0,
            Octave::O1 => crate::audio_processing::instruments::notes::Octave::O1,
            Octave::O2 => crate::audio_processing::instruments::notes::Octave::O2,
            Octave::O3 => crate::audio_processing::instruments::notes::Octave::O3,
            Octave::O4 => crate::audio_processing::instruments::notes::Octave::O4,
            Octave::O5 => crate::audio_processing::instruments::notes::Octave::O5,
            Octave::O6 => crate::audio_processing::instruments::notes::Octave::O6,
            Octave::O7 => crate::audio_processing::instruments::notes::Octave::O7,
            Octave::O8 => crate::audio_processing::instruments::notes::Octave::O8,
            Octave::O9 => crate::audio_processing::instruments::notes::Octave::O9,
            Octave::O10 => crate::audio_processing::instruments::notes::Octave::O10,
        }
    }
}

impl From<Octave> for crate::audio_processing::instruments::notes::Octave {
    fn from(o: Octave) -> Self {
        o.to_audio_octave()
    }
}

impl From<crate::audio_processing::instruments::notes::Octave> for Octave {
    fn from(o: crate::audio_processing::instruments::notes::Octave) -> Self {
        match o {
            crate::audio_processing::instruments::notes::Octave::OutOfRange => Octave::OutOfRange,
            crate::audio_processing::instruments::notes::Octave::O_1 => Octave::O_1,
            crate::audio_processing::instruments::notes::Octave::O0 => Octave::O0,
            crate::audio_processing::instruments::notes::Octave::O1 => Octave::O1,
            crate::audio_processing::instruments::notes::Octave::O2 => Octave::O2,
            crate::audio_processing::instruments::notes::Octave::O3 => Octave::O3,
            crate::audio_processing::instruments::notes::Octave::O4 => Octave::O4,
            crate::audio_processing::instruments::notes::Octave::O5 => Octave::O5,
            crate::audio_processing::instruments::notes::Octave::O6 => Octave::O6,
            crate::audio_processing::instruments::notes::Octave::O7 => Octave::O7,
            crate::audio_processing::instruments::notes::Octave::O8 => Octave::O8,
            crate::audio_processing::instruments::notes::Octave::O9 => Octave::O9,
            crate::audio_processing::instruments::notes::Octave::O10 => Octave::O10,
        }
    }
}
