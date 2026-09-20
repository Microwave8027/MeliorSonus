use rkyv::{Archive, Deserialize, Serialize};

/// Musical jump kinds (Da Capo, Dal Segno, etc.).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpKind {
    Segno,
    Coda,
    DaCapo,
    DalSegno,
    Fine,
    ToCoda,
    Other,
}

/// Repeat navigation jump (D.C., D.S., Segno, Coda, Fine).
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct RepeatJump {
    pub kind: JumpKind,
    pub text: Option<String>,
}
