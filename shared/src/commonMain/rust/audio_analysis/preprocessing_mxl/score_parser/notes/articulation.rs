use rkyv::{Archive, Deserialize, Serialize};

/// Musical note articulation.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteArticulation {
    Normal,
    Staccato,
    Tenuto,
    Legato,
    Marcato,
    Accent,
}

impl NoteArticulation {
    pub fn to_audio_articulation(&self) -> crate::audio_processing::instruments::notes::NoteArticulation {
        match self {
            NoteArticulation::Normal => crate::audio_processing::instruments::notes::NoteArticulation::Normal,
            NoteArticulation::Staccato => crate::audio_processing::instruments::notes::NoteArticulation::Staccato,
            NoteArticulation::Tenuto => crate::audio_processing::instruments::notes::NoteArticulation::Tenuto,
            NoteArticulation::Legato => crate::audio_processing::instruments::notes::NoteArticulation::Legato,
            NoteArticulation::Marcato | NoteArticulation::Accent => crate::audio_processing::instruments::notes::NoteArticulation::Marcato,
        }
    }
}

impl From<NoteArticulation> for crate::audio_processing::instruments::notes::NoteArticulation {
    fn from(a: NoteArticulation) -> Self {
        a.to_audio_articulation()
    }
}
