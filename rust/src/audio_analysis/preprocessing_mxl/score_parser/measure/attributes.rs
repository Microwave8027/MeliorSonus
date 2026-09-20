use rkyv::{Archive, Deserialize, Serialize};

/// Metadata attributes for a score measure.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MeasureAttributes {
    pub measure_number: String,
    pub full_measure: Option<bool>,
    pub divisions: u32,
}
