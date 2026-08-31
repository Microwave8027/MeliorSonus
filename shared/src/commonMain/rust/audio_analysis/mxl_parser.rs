use crate::audio_processing::instruments::notes::{Octave, Pitch};
use std::error::Error;

/// Symbolic representation of a reference note in a MusicXML score.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreNote {
    pub pitch: Pitch,
    pub octave: Octave,
    pub measure_index: usize,
    pub beat_position: f32,
    pub nominal_duration_beats: f32,
    pub expected_cents_offset: i8,
    pub is_rest: bool,
}

/// Parsed sheet music structure ready for performance alignment and scoring.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReferenceScore {
    pub title: String,
    pub composer: String,
    pub initial_bpm: f32,
    pub time_signature: (u8, u8), // e.g. (4, 4)
    pub notes: Vec<ScoreNote>,
}

/// MusicXML score parser (stubbed for `musicxml` crate integration).
pub struct MxlParser;

impl MxlParser {
    /// Parses an uncompressed `.musicxml` or compressed `.mxl` payload into a `ReferenceScore`.
    pub fn parse_bytes(_data: &[u8]) -> Result<ReferenceScore, Box<dyn Error>> {
        // Handoff stub for musicxml crate:
        // let score = musicxml::read_score_partwise(data)?;
        // Parse parts, measures, note elements and map to ScoreNote instances
        Ok(ReferenceScore {
            title: "Untitled Score".to_string(),
            composer: "Unknown".to_string(),
            initial_bpm: 120.0,
            time_signature: (4, 4),
            notes: Vec::new(),
        })
    }

    /// Convenience loader to parse a score directly from a file path.
    pub fn parse_file(_path: &str) -> Result<ReferenceScore, Box<dyn Error>> {
        // Handoff stub:
        // let bytes = std::fs::read(path)?;
        // Self::parse_bytes(&bytes)
        Ok(ReferenceScore::default())
    }
}
