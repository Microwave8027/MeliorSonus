use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Copy, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum PartListType {
    Parts(u32),
    Staves(u32),
}
