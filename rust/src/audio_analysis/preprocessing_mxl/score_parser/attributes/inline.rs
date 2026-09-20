use super::clef::ClefSign;
use super::pedal::PedalType;
use super::tempo::TempoChangeKind;
use super::wedge::WedgeType;
use crate::audio_analysis::score_parser::notes::DynamicLevel;
use rkyv::{Archive, Deserialize, Serialize};

/// Specific variant of an inline measure attribute.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum InlineAttributeKind {
    Dynamic {
        level: DynamicLevel,
        text: Option<String>,
    },
    TempoChange {
        bpm: Option<u32>,
        kind: TempoChangeKind,
        text: String,
    },
    Wedge {
        wedge_type: WedgeType,
    },
    Pedal {
        pedal_type: PedalType,
    },
    Clef {
        sign: ClefSign,
        line: Option<i8>,
        staff: Option<u32>,
    },
    KeySignature {
        fifths: i8,
    },
    TimeSignature {
        beats: u32,
        beat_type: u32,
    },
    Divisions(u32),
    OctaveShift {
        semitones: i8,
    },
    DirectionWords(String),
}

/// Attributes occurring sequentially within a measure, such as rits, dynamics, clef/key changes.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineMeasureAttributes {
    pub division_offset: u32,
    pub kind: InlineAttributeKind,
}
