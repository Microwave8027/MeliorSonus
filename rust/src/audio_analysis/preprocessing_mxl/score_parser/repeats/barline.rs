use rkyv::{Archive, Deserialize, Serialize};

/// Barline location within the measure.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarlineLocation {
    Left,
    Right,
    Middle,
}
