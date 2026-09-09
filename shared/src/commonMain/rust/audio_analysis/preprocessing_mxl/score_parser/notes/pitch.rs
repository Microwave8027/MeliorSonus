use rkyv::{Archive, Deserialize, Serialize};

/// Pitch representation matching equal temperament pitch classes.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl Pitch {
    pub fn from_midi(midi: u8) -> Self {
        match midi % 12 {
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

    pub fn to_audio_pitch(&self) -> crate::audio_processing::instruments::notes::Pitch {
        match self {
            Pitch::None => crate::audio_processing::instruments::notes::Pitch::None,
            Pitch::C => crate::audio_processing::instruments::notes::Pitch::C,
            Pitch::CsDf => crate::audio_processing::instruments::notes::Pitch::CsDf,
            Pitch::D => crate::audio_processing::instruments::notes::Pitch::D,
            Pitch::DsEf => crate::audio_processing::instruments::notes::Pitch::DsEf,
            Pitch::E => crate::audio_processing::instruments::notes::Pitch::E,
            Pitch::F => crate::audio_processing::instruments::notes::Pitch::F,
            Pitch::FsGf => crate::audio_processing::instruments::notes::Pitch::FsGf,
            Pitch::G => crate::audio_processing::instruments::notes::Pitch::G,
            Pitch::GsAf => crate::audio_processing::instruments::notes::Pitch::GsAf,
            Pitch::A => crate::audio_processing::instruments::notes::Pitch::A,
            Pitch::AsBf => crate::audio_processing::instruments::notes::Pitch::AsBf,
            Pitch::B => crate::audio_processing::instruments::notes::Pitch::B,
        }
    }
}

impl From<Pitch> for crate::audio_processing::instruments::notes::Pitch {
    fn from(p: Pitch) -> Self {
        p.to_audio_pitch()
    }
}

impl From<crate::audio_processing::instruments::notes::Pitch> for Pitch {
    fn from(p: crate::audio_processing::instruments::notes::Pitch) -> Self {
        match p {
            crate::audio_processing::instruments::notes::Pitch::None => Pitch::None,
            crate::audio_processing::instruments::notes::Pitch::C => Pitch::C,
            crate::audio_processing::instruments::notes::Pitch::CsDf => Pitch::CsDf,
            crate::audio_processing::instruments::notes::Pitch::D => Pitch::D,
            crate::audio_processing::instruments::notes::Pitch::DsEf => Pitch::DsEf,
            crate::audio_processing::instruments::notes::Pitch::E => Pitch::E,
            crate::audio_processing::instruments::notes::Pitch::F => Pitch::F,
            crate::audio_processing::instruments::notes::Pitch::FsGf => Pitch::FsGf,
            crate::audio_processing::instruments::notes::Pitch::G => Pitch::G,
            crate::audio_processing::instruments::notes::Pitch::GsAf => Pitch::GsAf,
            crate::audio_processing::instruments::notes::Pitch::A => Pitch::A,
            crate::audio_processing::instruments::notes::Pitch::AsBf => Pitch::AsBf,
            crate::audio_processing::instruments::notes::Pitch::B => Pitch::B,
        }
    }
}
