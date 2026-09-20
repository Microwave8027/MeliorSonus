use rkyv::{Archive, Deserialize, Serialize};

/// Kinds of tempo changes introduced as inline measure attributes.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempoChangeKind {
    Ritardando,
    Accelerando,
    ATempo,
    BpmChange,
    Other,
}
