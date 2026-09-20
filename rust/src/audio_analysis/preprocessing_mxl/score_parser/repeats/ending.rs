use rkyv::{Archive, Deserialize, Serialize};

/// Ending type for voltas (1st/2nd endings).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndingType {
    Start,
    Stop,
    Discontinue,
}

/// Multi-ending mark (1st, 2nd endings / voltas).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RepeatEnding {
    pub number: String,
    pub ending_type: EndingType,
    pub text: Option<String>,
}
