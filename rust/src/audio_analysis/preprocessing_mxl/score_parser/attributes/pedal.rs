use rkyv::{Archive, Deserialize, Serialize};

/// Piano damper / sostenuto pedal state.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PedalType {
    Start,
    Stop,
    Sostenuto,
    Change,
}
