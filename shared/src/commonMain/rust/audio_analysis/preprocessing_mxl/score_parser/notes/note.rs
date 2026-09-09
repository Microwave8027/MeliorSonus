use super::articulation::NoteArticulation;
use super::dynamic::DynamicLevel;
use super::octave::Octave;
use super::pitch::Pitch;
use super::tie::TieType;
use rkyv::{Archive, Deserialize, Serialize};

/// Represents the onset information of a musical note, comparable to `StartNote` in audio processing.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct StartNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub alter: i8,
    pub tonality_offset: i8,
    pub velocity: u8,
    pub dynamic: DynamicLevel,
    pub start_division: u32,
    pub duration_divisions: u32,
    pub midi_note: u8,
    pub voice: u32,
    pub staff: u32,
    pub is_grace: bool,
}

/// Represents the completed evaluation of a musical note, comparable to `EndNote` in audio processing.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct EndNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub alter: i8,
    pub tonality_offset: i8,
    pub duration_divisions: u32,
    pub start_division: u32,
    pub end_division: u32,
    pub articulation: NoteArticulation,
    pub dynamic: DynamicLevel,
    pub velocity: u8,
    pub tie: TieType,
    pub midi_note: u8,
    pub voice: u32,
    pub staff: u32,
    pub is_grace: bool,
}

/// Enum representing either note onset or note completion.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
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

    pub fn midi_note(&self) -> u8 {
        match self {
            Notes::Start(s) => s.midi_note,
            Notes::End(e) => e.midi_note,
        }
    }
}

impl From<StartNote> for Notes {
    fn from(s: StartNote) -> Self {
        Notes::Start(s)
    }
}

impl From<EndNote> for Notes {
    fn from(e: EndNote) -> Self {
        Notes::End(e)
    }
}

/// A cluster or simultaneous group of notes occurring at a specific division timestamp.
/// Instead of distinguishing between single notes and clusters with an enum, every note event
/// is represented uniformly as a vector of `StartNote`s and `EndNote`s.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct NoteCluster {
    pub start_division: u32,
    pub start_notes: Vec<StartNote>,
    pub end_notes: Vec<EndNote>,
}

impl NoteCluster {
    pub fn new(start_division: u32) -> Self {
        Self {
            start_division,
            start_notes: Vec::new(),
            end_notes: Vec::new(),
        }
    }

    pub fn is_chord(&self) -> bool {
        self.start_notes.len() > 1
    }

    pub fn len(&self) -> usize {
        self.start_notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.start_notes.is_empty()
    }

    pub fn max_end_division(&self) -> u32 {
        self.end_notes
            .iter()
            .map(|n| n.end_division)
            .max()
            .unwrap_or(self.start_division)
    }
}

pub type MxlNotes = NoteCluster;
