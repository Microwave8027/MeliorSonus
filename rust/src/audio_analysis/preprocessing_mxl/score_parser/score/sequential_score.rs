use crate::audio_analysis::JumpKind;
use crate::audio_analysis::RepeatVariant;
use crate::audio_analysis::mxl_metadata::PartListType;
use crate::audio_analysis::score_parser::repeats::EndingType;
use crate::audio_analysis::score_parser::measure::ScoreMeasure;
use crate::audio_analysis::score_parser::notes::{EndNote, NoteCluster, StartNote};
use crate::audio_analysis::score_parser::parser::{
    ParserState, iterate_over_measures, iterate_over_measures_for_instrument,
    iterate_over_measures_with_target_part, iterate_over_measures_with_target_parts,
    parse_measure, parse_measure_with_target_part, resolve_target_part_index,
    resolve_target_part_indices,
};
use musicxml::elements::Measure;
use rkyv::{Archive, Deserialize, Serialize};
use std::collections::HashMap;
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

    pub fn from_instrument(
        measures: &Vec<Measure>,
        parts: &PartListType,
        instruments: &[String],
        target_instrument: &str,
    ) -> Self {
        iterate_over_measures_for_instrument(measures, parts, instruments, target_instrument)
    }

    pub fn from_piano(
        measures: &Vec<Measure>,
        parts: &PartListType,
        instruments: &[String],
    ) -> Self {
        Self::from_instrument(measures, parts, instruments, "piano")
    }

    pub fn from_target_part(
        measures: &Vec<Measure>,
        parts: &PartListType,
        target_part_index: Option<u32>,
    ) -> Self {
        iterate_over_measures_with_target_part(measures, parts, target_part_index)
    }

    pub fn from_target_parts(
        measures: &Vec<Measure>,
        parts: &PartListType,
        target_part_indices: Option<&[u32]>,
    ) -> Self {
        iterate_over_measures_with_target_parts(measures, parts, target_part_indices)
    }

    pub fn iterate_over_measures(measures: &Vec<Measure>, parts: &PartListType) -> Self {
        iterate_over_measures(measures, parts)
    }

    pub fn iterate_over_measures_for_instrument(
        measures: &Vec<Measure>,
        parts: &PartListType,
        instruments: &[String],
        target_instrument: &str,
    ) -> Self {
        iterate_over_measures_for_instrument(measures, parts, instruments, target_instrument)
    }

    pub fn parse_measure(measure: &Measure, state: &mut ParserState) -> ScoreMeasure {
        parse_measure(measure, state)
    }

    pub fn parse_measure_with_target_part(
        measure: &Measure,
        state: &mut ParserState,
        target_part_index: Option<u32>,
    ) -> ScoreMeasure {
        parse_measure_with_target_part(measure, state, target_part_index)
    }

    pub fn total_measures(&self) -> usize {
        self.notes.len()
    }

    pub fn all_notes(&self) -> Vec<&NoteCluster> {
        self.notes.iter().flat_map(|m| m.notes()).collect()
    }

    pub fn all_start_notes(&self) -> Vec<&StartNote> {
        self.notes
            .iter()
            .flat_map(|m| m.all_start_notes())
            .collect()
    }

    pub fn all_end_notes(&self) -> Vec<&EndNote> {
        self.notes.iter().flat_map(|m| m.all_end_notes()).collect()
    }

    /// Unrolls repeats, voltas, and jumps into a chronological playback sequence of measures matching audio performance order.
    pub fn expand_playback_sequence(&self) -> Vec<&ScoreMeasure> {
        if self.notes.is_empty() {
            return Vec::new();
        }

        let mut sequence: Vec<&ScoreMeasure> = Vec::new();
        let total = self.notes.len();
        let max_iterations = total.saturating_mul(16).max(1024);

        let mut idx = 0;
        let mut repeat_start = 0;
        let mut current_pass: u32 = 1;
        let mut in_ending_section = false;
        let mut repeat_counts: HashMap<usize, u32> = HashMap::new();
        let mut has_jumped = false;
        let mut segno_idx: Option<usize> = None;
        let mut coda_idx: Option<usize> = None;

        // Pre-scan for Segno and Coda positions
        for (i, m) in self.notes.iter().enumerate() {
            for rep in m.repeats() {
                if let RepeatVariant::Jump(j) = rep {
                    match j.kind {
                        JumpKind::Segno => {
                            if segno_idx.is_none() {
                                segno_idx = Some(i);
                            }
                        }
                        JumpKind::Coda => {
                            if coda_idx.is_none() {
                                coda_idx = Some(i);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let mut steps = 0;
        while idx < total && steps < max_iterations {
            steps += 1;
            let measure = &self.notes[idx];

            // 1. Check if the measure has an alternative ending (volta)
            let start_ending = measure.repeats().find_map(|r| match r {
                RepeatVariant::Ending(e) if e.ending_type == EndingType::Start => Some(e),
                _ => None,
            });

            if let Some(ending) = start_ending {
                if !ending_includes_pass(&ending.number, current_pass) {
                    // Find next matching ending or skip past the end of the entire volta group
                    let mut target_idx = idx + 1;
                    while target_idx < total {
                        let next_m = &self.notes[target_idx];
                        if let Some(next_ending) = next_m.repeats().find_map(|r| match r {
                            RepeatVariant::Ending(e) if e.ending_type == EndingType::Start => Some(e),
                            _ => None,
                        }) {
                            if ending_includes_pass(&next_ending.number, current_pass) {
                                break;
                            }
                        } else {
                            let prev_had_stop = self.notes[target_idx - 1].repeats().any(|r| match r {
                                RepeatVariant::Ending(e) => {
                                    matches!(e.ending_type, EndingType::Stop | EndingType::Discontinue)
                                }
                                RepeatVariant::End(_) => true,
                                _ => false,
                            });
                            if prev_had_stop {
                                break;
                            }
                        }
                        target_idx += 1;
                    }

                    idx = target_idx;
                    continue;
                }
            }

            // 2. Track ending section transitions
            let is_in_ending = measure.repeats().any(|r| r.is_ending());
            if in_ending_section && !is_in_ending {
                in_ending_section = false;
                current_pass = 1;
                repeat_start = idx;
                repeat_counts.clear();
            }
            if is_in_ending {
                in_ending_section = true;
            }

            sequence.push(measure);

            let mut jump_target: Option<usize> = None;
            let mut handled_start = false;
            let mut handled_end = false;

            let has_ending_stop = measure.repeats().any(|r| match r {
                RepeatVariant::Ending(e) => {
                    matches!(e.ending_type, EndingType::Stop | EndingType::Discontinue)
                }
                _ => false,
            });
            let has_repeat_end = measure.repeats().any(|r| r.is_end());

            if has_ending_stop && !has_repeat_end {
                in_ending_section = false;
                current_pass = 1;
                repeat_start = idx + 1;
                repeat_counts.clear();
            }

            for rep in measure.repeats() {
                match rep {
                    RepeatVariant::Start(_) if !handled_start => {
                        handled_start = true;
                        if idx != repeat_start {
                            repeat_start = idx;
                            current_pass = 1;
                            in_ending_section = false;
                        }
                    }
                    RepeatVariant::End(end) if !handled_end => {
                        handled_end = true;
                        let times_to_play = end.times.max(2);
                        let count = repeat_counts.entry(idx).or_insert(1);
                        if *count < times_to_play {
                            *count += 1;
                            current_pass += 1;
                            in_ending_section = false;
                            jump_target = Some(repeat_start);
                        } else {
                            repeat_counts.remove(&idx);
                            repeat_start = idx + 1;
                            current_pass = 1;
                            in_ending_section = false;
                        }
                    }
                    RepeatVariant::Jump(j) => match j.kind {
                        JumpKind::DaCapo if !has_jumped => {
                            has_jumped = true;
                            current_pass = 1;
                            in_ending_section = false;
                            repeat_start = 0;
                            jump_target = Some(0);
                        }
                        JumpKind::DalSegno if !has_jumped => {
                            if let Some(s_idx) = segno_idx {
                                has_jumped = true;
                                current_pass = 1;
                                in_ending_section = false;
                                repeat_start = s_idx;
                                jump_target = Some(s_idx);
                            }
                        }
                        JumpKind::Fine if has_jumped => {
                            return sequence;
                        }
                        JumpKind::ToCoda if has_jumped => {
                            if let Some(c_idx) = coda_idx {
                                jump_target = Some(c_idx);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if let Some(target) = jump_target {
                idx = target;
            } else {
                idx += 1;
            }
        }

        sequence
    }

    /// Returns all start notes across the chronologically expanded playback sequence.
    pub fn unrolled_start_notes(&self) -> Vec<&StartNote> {
        self.expand_playback_sequence()
            .iter()
            .flat_map(|m| m.all_start_notes())
            .collect()
    }

    /// Returns all non-tied start notes across the expanded playback sequence (only notes with an audio strike onset).
    pub fn unrolled_strike_notes(&self) -> Vec<&StartNote> {
        self.unrolled_start_notes()
            .into_iter()
            .filter(|n| !n.is_tied_continuation())
            .collect()
    }

    /// Returns all end notes across the chronologically expanded playback sequence.
    pub fn unrolled_end_notes(&self) -> Vec<&EndNote> {
        self.expand_playback_sequence()
            .iter()
            .flat_map(|m| m.all_end_notes())
            .collect()
    }
}

pub fn ending_includes_pass(number_str: &str, pass: u32) -> bool {
    for part in number_str.split(',') {
        let part = part.trim().trim_end_matches('.');
        if let Some((start_s, end_s)) = part.split_once('-') {
            let start = start_s.trim().trim_end_matches('.').parse::<u32>();
            let end = end_s.trim().trim_end_matches('.').parse::<u32>();
            if let (Ok(s), Ok(e)) = (start, end) {
                if pass >= s && pass <= e {
                    return true;
                }
            }
        } else if let Ok(n) = part.parse::<u32>() {
            if n == pass {
                return true;
            }
        }
    }
    false
}
