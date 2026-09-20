use super::attributes::MeasureAttributes;
use super::content::ScoreContent;
use crate::audio_analysis::score_parser::attributes::{InlineMeasureAttributes, RestInfo};
use crate::audio_analysis::score_parser::notes::{EndNote, NoteCluster, StartNote};
use crate::audio_analysis::score_parser::repeats::RepeatVariant;
use rkyv::{Archive, Deserialize, Serialize};

/// Represents a single parsed measure within the sequential score.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ScoreMeasure {
    pub content: Vec<ScoreContent>,
    pub attributes: MeasureAttributes,
}

impl ScoreMeasure {
    pub fn notes(&self) -> impl Iterator<Item = &NoteCluster> {
        self.content.iter().filter_map(|c| match c {
            ScoreContent::Notes(n) => Some(n),
            _ => None,
        })
    }

    pub fn inline_attributes(&self) -> impl Iterator<Item = &InlineMeasureAttributes> {
        self.content.iter().filter_map(|c| match c {
            ScoreContent::InlineMeasureAttributes(a) => Some(a),
            _ => None,
        })
    }

    pub fn repeats(&self) -> impl Iterator<Item = &RepeatVariant> {
        self.content.iter().filter_map(|c| match c {
            ScoreContent::Repeat(r) => Some(r),
            _ => None,
        })
    }

    pub fn rests(&self) -> impl Iterator<Item = &RestInfo> {
        self.content.iter().filter_map(|c| match c {
            ScoreContent::Rest(r) => Some(r),
            _ => None,
        })
    }

    pub fn all_start_notes(&self) -> Vec<&StartNote> {
        self.notes().flat_map(|c| &c.start_notes).collect()
    }

    pub fn all_end_notes(&self) -> Vec<&EndNote> {
        self.notes().flat_map(|c| &c.end_notes).collect()
    }
}
