use musicxml::datatypes::YesNo;
use musicxml::elements::Measure;
use rkyv::{Archive, Deserialize, Serialize};

/// Metadata attributes for a score measure.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MeasureAttributes {
    pub measure_number: String,
    pub full_measure: Option<bool>,
}

impl MeasureAttributes {
    pub fn extract_measure_attributes(measure: &Measure) -> Self {
        Self {
            measure_number: measure.attributes.number.clone().to_string(),
            full_measure: match measure.attributes.implicit {
                Some(YesNo::Yes) => Some(true),
                Some(YesNo::No) => Some(false),
                None => None,
            },
        }
    }
}
