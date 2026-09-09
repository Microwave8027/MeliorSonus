use crate::audio_analysis::score_parser::attributes::{InlineMeasureAttributes, RestInfo};
use crate::audio_analysis::score_parser::notes::NoteCluster;
use crate::audio_analysis::score_parser::repeats::RepeatVariant;
use rkyv::{Archive, Deserialize, Serialize};

/// Measure sequential content element.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ScoreContent {
    Notes(NoteCluster),
    InlineMeasureAttributes(InlineMeasureAttributes),
    Repeat(RepeatVariant),
    Rest(RestInfo),
}

pub type MeasureContent = ScoreContent;
