use super::barline::BarlineLocation;
use rkyv::{Archive, Deserialize, Serialize};

/// Start of a repeated section (`|:`).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RepeatStart {
    pub location: BarlineLocation,
    pub winged: Option<bool>,
}

/// End of a repeated section (`:|`).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RepeatEnd {
    pub location: BarlineLocation,
    pub times: u32,
    pub after_jump: bool,
    pub winged: Option<bool>,
}
