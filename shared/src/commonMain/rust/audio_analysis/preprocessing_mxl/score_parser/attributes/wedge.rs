use rkyv::{Archive, Deserialize, Serialize};

/// Crescendo / Diminuendo wedges.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum WedgeType {
    Crescendo,
    Diminuendo,
    Stop,
}
