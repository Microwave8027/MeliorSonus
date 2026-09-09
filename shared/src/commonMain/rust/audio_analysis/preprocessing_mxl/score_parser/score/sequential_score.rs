use crate::audio_analysis::mxl_metadata::PartListType;
use crate::audio_analysis::score_parser::measure::ScoreMeasure;
use crate::audio_analysis::score_parser::notes::{EndNote, NoteCluster, StartNote};
use crate::audio_analysis::score_parser::parser::{iterate_over_measures, parse_measure, ParserState};
use musicxml::elements::Measure;
use rkyv::{Archive, Deserialize, Serialize};
use std::ops::Deref;

/// Wrapper for a vector of parsed score measures, implements deref.
#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SequentialMusicScore {
    pub notes: Vec<ScoreMeasure>,
}

impl Deref for SequentialMusicScore {
    type Target = Vec<ScoreMeasure>;

    fn deref(&self) -> &Self::Target {
        &self.notes
    }
}

impl SequentialMusicScore {
    pub fn from(measures: &Vec<Measure>, parts: &PartListType) -> Self {
        iterate_over_measures(measures, parts)
    }

    pub fn iterate_over_measures(measures: &Vec<Measure>, parts: &PartListType) -> Self {
        iterate_over_measures(measures, parts)
    }

    pub fn parse_measure(measure: &Measure, state: &mut ParserState) -> ScoreMeasure {
        parse_measure(measure, state)
    }

    pub fn total_measures(&self) -> usize {
        self.notes.len()
    }

    pub fn all_notes(&self) -> Vec<&NoteCluster> {
        self.notes.iter().flat_map(|m| m.notes()).collect()
    }

    pub fn all_start_notes(&self) -> Vec<&StartNote> {
        self.notes.iter().flat_map(|m| m.all_start_notes()).collect()
    }

    pub fn all_end_notes(&self) -> Vec<&EndNote> {
        self.notes.iter().flat_map(|m| m.all_end_notes()).collect()
    }
}
