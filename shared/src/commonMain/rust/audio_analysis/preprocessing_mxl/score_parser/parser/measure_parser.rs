use super::attributes_handler::handle_attributes;
use super::barline_handler::handle_barline;
use super::direction_handler::handle_direction;
use super::note_handler::handle_note;
use super::sound_handler::handle_sound;
use super::state::ParserState;
use crate::audio_analysis::mxl_metadata::PartListType;
use crate::audio_analysis::score_parser::attributes::{InlineMeasureAttributes, RestInfo};
use crate::audio_analysis::score_parser::measure::{MeasureAttributes, ScoreContent, ScoreMeasure};
use crate::audio_analysis::score_parser::notes::{EndNote, NoteCluster, StartNote};
use crate::audio_analysis::score_parser::repeats::{BarlineLocation, RepeatVariant};
use crate::audio_analysis::score_parser::score::SequentialMusicScore;
use musicxml::elements::{Measure, MeasureElement, Part, PartElement};
use std::collections::BTreeMap;

pub fn iterate_over_measures(measures: &Vec<Measure>, _parts: &PartListType) -> SequentialMusicScore {
    let mut state = ParserState {
        current_divisions: 1,
        current_bpm: None,
        current_dynamic: None,
    };

    let notes: Vec<ScoreMeasure> = measures
        .iter()
        .map(|measure| parse_measure(measure, &mut state))
        .collect();

    SequentialMusicScore { notes }
}

pub fn parse_measure(measure: &Measure, state: &mut ParserState) -> ScoreMeasure {
    let measure_attributes = MeasureAttributes::extract_measure_attributes(measure);

    let mut raw_notes: Vec<(u32, StartNote, EndNote)> = Vec::new();
    let mut inline_attrs: Vec<InlineMeasureAttributes> = Vec::new();
    let mut repeats: Vec<(u32, RepeatVariant)> = Vec::new();
    let mut rests: Vec<RestInfo> = Vec::new();

    // Process top-level measure elements (barlines, attributes, directions)
    for elem in &measure.content {
        match elem {
            MeasureElement::Barline(barline) => {
                handle_barline(barline, 0, &mut repeats);
            }
            MeasureElement::Direction(dir) => {
                handle_direction(dir, 0, state, &mut inline_attrs, &mut repeats);
            }
            MeasureElement::Attributes(attrs) => {
                handle_attributes(attrs, 0, state, &mut inline_attrs);
            }
            MeasureElement::Sound(sound) => {
                handle_sound(sound, 0, state, &mut inline_attrs, &mut repeats);
            }
            _ => {}
        }
    }

    // Collect part elements
    let parts: Vec<&Part> = measure
        .content
        .iter()
        .filter_map(|part| match part {
            MeasureElement::Part(p) => Some(p),
            _ => None,
        })
        .collect();

    if parts.is_empty() {
        // Direct elements inside measure
        let mut current_position: u32 = 0;
        let mut last_note_start: u32 = 0;
        for elem in &measure.content {
            match elem {
                MeasureElement::Attributes(attrs) => {
                    handle_attributes(attrs, current_position, state, &mut inline_attrs);
                }
                MeasureElement::Direction(dir) => {
                    handle_direction(dir, current_position, state, &mut inline_attrs, &mut repeats);
                }
                MeasureElement::Sound(sound) => {
                    handle_sound(sound, current_position, state, &mut inline_attrs, &mut repeats);
                }
                MeasureElement::Backup(backup) => {
                    let dur = *backup.content.duration.content;
                    current_position = current_position.saturating_sub(dur);
                    last_note_start = current_position;
                }
                MeasureElement::Forward(forward) => {
                    let dur = *forward.content.duration.content;
                    current_position = current_position.saturating_add(dur);
                    last_note_start = current_position;
                }
                MeasureElement::Note(note) => {
                    handle_note(note, &mut current_position, &mut last_note_start, state, &mut raw_notes, &mut rests);
                }
                _ => {}
            }
        }
    } else {
        // Traverse each part
        for part in parts {
            let mut current_position: u32 = 0;
            let mut last_note_start: u32 = 0;
            for element in &part.content {
                match element {
                    PartElement::Attributes(attrs) => {
                        handle_attributes(attrs, current_position, state, &mut inline_attrs);
                    }
                    PartElement::Direction(dir) => {
                        handle_direction(dir, current_position, state, &mut inline_attrs, &mut repeats);
                    }
                    PartElement::Sound(sound) => {
                        handle_sound(sound, current_position, state, &mut inline_attrs, &mut repeats);
                    }
                    PartElement::Backup(backup) => {
                        let dur = *backup.content.duration.content;
                        current_position = current_position.saturating_sub(dur);
                        last_note_start = current_position;
                    }
                    PartElement::Forward(forward) => {
                        let dur = *forward.content.duration.content;
                        current_position = current_position.saturating_add(dur);
                        last_note_start = current_position;
                    }
                    PartElement::Barline(barline) => {
                        handle_barline(barline, current_position, &mut repeats);
                    }
                    PartElement::Note(note) => {
                        handle_note(note, &mut current_position, &mut last_note_start, state, &mut raw_notes, &mut rests);
                    }
                    _ => {}
                }
            }
        }
    }

    // Group notes into clusters by start_division
    let mut clusters_map: BTreeMap<u32, (Vec<StartNote>, Vec<EndNote>)> = BTreeMap::new();
    for (start_div, start_note, end_note) in raw_notes {
        let entry = clusters_map
            .entry(start_div)
            .or_insert_with(|| (Vec::new(), Vec::new()));
        entry.0.push(start_note);
        entry.1.push(end_note);
    }

    // Combine into ordered sequence:
    // Priority: Left repeats (0) -> Attributes (1) -> Notes (2) -> Rests (3) -> Right repeats (4)
    let mut items: Vec<((u32, u8), ScoreContent)> = Vec::new();

    for (div, (start_notes, end_notes)) in clusters_map {
        let cluster = NoteCluster {
            start_division: div,
            start_notes,
            end_notes,
        };
        items.push(((div, 2), ScoreContent::Notes(cluster)));
    }

    for attr in inline_attrs {
        let div = attr.division_offset;
        items.push(((div, 1), ScoreContent::InlineMeasureAttributes(attr)));
    }

    for rest in rests {
        let div = rest.start_division;
        items.push(((div, 3), ScoreContent::Rest(rest)));
    }

    for (div, rep) in repeats {
        let (key_div, priority) = match &rep {
            RepeatVariant::Start(s) if s.location == BarlineLocation::Left => (0, 0),
            _ => (div.max(u32::MAX - 1), 4),
        };
        items.push(((key_div, priority), ScoreContent::Repeat(rep)));
    }

    items.sort_by_key(|(k, _)| *k);
    let content = items.into_iter().map(|(_, c)| c).collect();

    ScoreMeasure {
        content,
        attributes: measure_attributes,
    }
}
