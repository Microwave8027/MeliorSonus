use crate::audio_analysis::score_parser::notes::DynamicLevel;

/// Internal parser state carrying ongoing tempo, dynamic, division resolution, transposition, and pedal across measures.
#[derive(Clone, Debug)]
pub struct ParserState {
    pub current_divisions: u32,
    pub current_bpm: Option<u32>,
    pub current_dynamic: Option<DynamicLevel>,
    pub current_transpose_semitones: i8,
    pub current_octave_shift: i8,
    pub is_pedal_active: bool,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            current_divisions: 1,
            current_bpm: None,
            current_dynamic: None,
            current_transpose_semitones: 0,
            current_octave_shift: 0,
            is_pedal_active: false,
        }
    }
}
