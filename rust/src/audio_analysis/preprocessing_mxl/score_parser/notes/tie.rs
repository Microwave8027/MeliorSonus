use rkyv::{Archive, Deserialize, Serialize};

/// Note tie state.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TieType {
    None,
    Start,
    Stop,
    Continue,
}
