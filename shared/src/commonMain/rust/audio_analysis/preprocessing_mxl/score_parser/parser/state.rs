use crate::audio_analysis::score_parser::notes::DynamicLevel;

/// Internal parser state carrying ongoing tempo, dynamic, and division resolution across measures.
#[derive(Default, Clone, Debug)]
pub struct ParserState {
    pub current_divisions: u32,
    pub current_bpm: Option<u32>,
    pub current_dynamic: Option<DynamicLevel>,
}
