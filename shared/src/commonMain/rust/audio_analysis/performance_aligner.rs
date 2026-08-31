use super::mxl_parser::{ReferenceScore, ScoreNote};
use crate::audio_processing::instruments::notes::EndNote;
use std::error::Error;

/// Classification of a student's played note relative to the reference score.
#[derive(Clone, Debug, PartialEq)]
pub enum AlignmentStatus {
    ExactMatch,
    IntonationError { cents_delta: i8 },
    RhythmError { delta_beats: f32 },
    WrongPitch,
    MissedScoreNote,
    ExtraPlayedNote,
}

/// Alignment matching pairing a played note with a score note.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteAlignmentMatch {
    pub score_note: Option<ScoreNote>,
    pub played_note: Option<EndNote>,
    pub status: AlignmentStatus,
    pub pitch_score: f32,  // 0.0 - 100.0
    pub rhythm_score: f32, // 0.0 - 100.0
}

/// Summary result of aligning an entire practice run against the reference sheet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AlignmentResult {
    pub matches: Vec<NoteAlignmentMatch>,
    pub overall_pitch_accuracy: f32,
    pub overall_rhythm_accuracy: f32,
    pub total_missed_notes: usize,
    pub total_extra_notes: usize,
}

/// Offline / Non-Real-Time Score Following engine (stubbed for `fastdtw` crate).
pub struct PerformanceAligner;

impl PerformanceAligner {
    /// Aligns a recorded sequence of `EndNote` instances with a `ReferenceScore` using Fast Dynamic Time Warping.
    pub fn align(
        _performed_notes: &[EndNote],
        _reference_score: &ReferenceScore,
    ) -> Result<AlignmentResult, Box<dyn Error>> {
        // Handoff stub for fastdtw:
        // 1. Convert performed_notes and reference_score.notes into comparable feature vectors (time, pitch).
        // 2. Compute FastDTW warping path.
        // 3. Classify matches into AlignmentStatus items and compute accuracy percentages.
        Ok(AlignmentResult {
            matches: Vec::new(),
            overall_pitch_accuracy: 100.0,
            overall_rhythm_accuracy: 100.0,
            total_missed_notes: 0,
            total_extra_notes: 0,
        })
    }
}
