use rkyv::{Archive, Deserialize, Serialize};

/// Musical ornament indications (trills, turns, mordents, tremolos).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrnamentKind {
    Trill,
    Turn,
    InvertedTurn,
    Mordent,
    InvertedMordent,
    Tremolo,
    Other,
}
