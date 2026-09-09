use rkyv::{Archive, Deserialize, Serialize};

/// Rest information occurring within a measure.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RestInfo {
    pub start_division: u32,
    pub duration: u32,
    pub voice: u32,
    pub staff: u32,
}
