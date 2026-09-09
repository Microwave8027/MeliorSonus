use rkyv::{Archive, Deserialize, Serialize};

/// Clef sign indicators.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClefSign {
    G,
    F,
    C,
    Percussion,
    TAB,
    Other,
}
