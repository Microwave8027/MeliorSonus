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
use crate::audio_analysis::score_parser::repeats::{BarlineLocation, EndingType, RepeatVariant};
use crate::audio_analysis::score_parser::score::SequentialMusicScore;
use musicxml::datatypes::YesNo;
use musicxml::elements::{GraceType, Measure, MeasureElement, Note, NoteType, Part, PartElement};
use std::collections::BTreeMap;

pub fn iterate_over_measures(
    measures: &Vec<Measure>,
    parts: &PartListType,
) -> SequentialMusicScore {
    iterate_over_measures_with_target_part(measures, parts, None)
}

/// Iterates over the measures matching the parts against the target
pub fn iterate_over_measures_for_instrument(
    measures: &Vec<Measure>,
    parts: &PartListType,
    instruments: &[String],
    target_instrument: &str,
) -> SequentialMusicScore {
    let indices = resolve_target_part_indices(instruments, target_instrument);
    let target_indices = if indices.is_empty() {
        None
    } else {
        Some(indices.as_slice())
    };
    iterate_over_measures_with_target_parts(measures, parts, target_indices)
}

/// Iterates assuming the target_part_index is valid
pub fn iterate_over_measures_with_target_part(
    measures: &Vec<Measure>,
    parts: &PartListType,
    target_part_index: Option<u32>,
) -> SequentialMusicScore {
    let indices_vec = target_part_index.map(|idx| vec![idx]);
    let target_indices = indices_vec.as_deref();
    iterate_over_measures_with_target_parts(measures, parts, target_indices)
}

pub fn iterate_over_measures_with_target_parts(
    measures: &Vec<Measure>,
    _parts: &PartListType,
    target_part_indices: Option<&[u32]>,
) -> SequentialMusicScore {
    let mut state = ParserState {
        current_divisions: 1,
        current_bpm: None,
        current_dynamic: None,
        current_transpose_semitones: 0,
        current_octave_shift: 0,
        is_pedal_active: false,
    };
    let notes: Vec<ScoreMeasure> = measures
        .iter()
        .map(|measure| parse_measure_with_target_parts(measure, &mut state, target_part_indices))
        .collect();

    SequentialMusicScore { notes }
}

/// Matches target instrument against instrument names.
/// Matches all part indices where the lowercase target instrument query appears as a substring.
/// Returns all matching part indices, enabling instruments split across multiple parts (e.g. Piano RH/LH) to be included together.
pub fn resolve_target_part_indices(instruments: &[String], target_instrument: &str) -> Vec<u32> {
    let target_lower = target_instrument.to_lowercase();
    let matches: Vec<u32> = instruments
        .iter()
        .enumerate()
        .filter_map(|(idx, name)| {
            if name.to_lowercase().contains(&target_lower) {
                Some(idx as u32)
            } else {
                None
            }
        })
        .collect();

    if !matches.is_empty() {
        matches
    } else if instruments.len() == 1 {
        vec![0]
    } else {
        Vec::new()
    }
}

/// Legacy helper for single-part resolution.
#[deprecated]
pub fn resolve_target_part_index(instruments: &[String], target_instrument: &str) -> Option<u32> {
    resolve_target_part_indices(instruments, target_instrument)
        .into_iter()
        .next()
}

pub fn parse_measure(measure: &Measure, state: &mut ParserState) -> ScoreMeasure {
    parse_measure_with_target_parts(measure, state, None)
}

pub fn parse_measure_with_target_part(
    measure: &Measure,
    state: &mut ParserState,
    target_part_index: Option<u32>,
) -> ScoreMeasure {
    let indices_slice = target_part_index.map(|idx| [idx]);
    parse_measure_with_target_parts(
        measure,
        state,
        indices_slice.as_ref().map(|arr| arr.as_slice()),
    )
}

pub fn parse_measure_with_target_parts(
    measure: &Measure,
    state: &mut ParserState,
    target_part_indices: Option<&[u32]>,
) -> ScoreMeasure {
    let mut raw_notes: Vec<(u32, StartNote, EndNote)> = Vec::new();
    let mut inline_attrs: Vec<InlineMeasureAttributes> = Vec::new();
    let mut repeats: Vec<(u32, RepeatVariant)> = Vec::new();
    let mut rests: Vec<RestInfo> = Vec::new();

    // Collects the instrument parts from the music
    let parts: Vec<&Part> = measure
        .content
        .iter()
        .filter_map(|part| match part {
            MeasureElement::Part(p) => Some(p),
            _ => None,
        })
        .collect();

    // If multi-part score, process top-level measure elements (barlines, directions) above parts at division 0
    if !parts.is_empty() {
        let mut top_pos = 0;
        let mut top_last = 0;
        for elem in &measure.content {
            if let Some(score_elem) = ScoreElement::from_measure_element(elem) {
                process_element(
                    score_elem,
                    &mut top_pos,
                    &mut top_last,
                    state,
                    0,
                    target_part_indices,
                    &mut inline_attrs,
                    &mut repeats,
                    &mut raw_notes,
                    &mut rests,
                );
            }
        }
    }

    // If no parts exist, measure.content itself is treated as part 0
    let part_streams: Vec<(u32, Vec<ScoreElement>)> = if parts.is_empty() {
        vec![(
            0,
            measure
                .content
                .iter()
                .filter_map(ScoreElement::from_measure_element)
                .collect(),
        )]
    } else {
        parts
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                (
                    idx as u32,
                    p.content
                        .iter()
                        .filter_map(ScoreElement::from_part_element)
                        .collect(),
                )
            })
            .collect()
    };

    for (part_idx, stream) in part_streams {
        let mut current_position: u32 = 0;
        let mut last_note_start: u32 = 0;
        for elem in stream {
            process_element(
                elem,
                &mut current_position,
                &mut last_note_start,
                state,
                part_idx,
                target_part_indices,
                &mut inline_attrs,
                &mut repeats,
                &mut raw_notes,
                &mut rests,
            );
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

    let mut unique_inline_attrs: Vec<InlineMeasureAttributes> = Vec::new();
    for attr in inline_attrs {
        if !unique_inline_attrs.contains(&attr) {
            unique_inline_attrs.push(attr);
        }
    }
    for attr in unique_inline_attrs {
        let div = attr.division_offset;
        items.push(((div, 1), ScoreContent::InlineMeasureAttributes(attr)));
    }

    for rest in rests {
        let div = rest.start_division;
        items.push(((div, 3), ScoreContent::Rest(rest)));
    }

    let mut unique_repeats: Vec<(u32, RepeatVariant)> = Vec::new();
    for (div, rep) in repeats {
        if !unique_repeats.iter().any(|(d, r)| *d == div && r == &rep) {
            unique_repeats.push((div, rep));
        }
    }
    for (div, rep) in unique_repeats {
        let (key_div, priority) = match &rep {
            RepeatVariant::Start(s) if s.location == BarlineLocation::Left => (0, 0),
            RepeatVariant::Ending(e) if e.ending_type == EndingType::Start => (0, 0),
            _ => (div.max(u32::MAX - 1), 4),
        };
        items.push(((key_div, priority), ScoreContent::Repeat(rep)));
    }

    items.sort_by_key(|(k, _)| *k);
    let content = items.into_iter().map(|(_, c)| c).collect();

    let measure_attributes = extract_measure_attributes(measure, state.current_divisions);

    ScoreMeasure {
        content,
        attributes: measure_attributes,
    }
}

pub fn extract_measure_attributes(measure: &Measure, divisions: u32) -> MeasureAttributes {
    MeasureAttributes {
        measure_number: measure.attributes.number.clone().to_string(),
        full_measure: match measure.attributes.implicit {
            Some(YesNo::Yes) => Some(false), // implicit="yes" indicates a pickup/anacrusis (incomplete) measure
            Some(YesNo::No) => Some(true),
            None => None,
        },
        divisions,
    }
}

fn advance_note_position(note: &Note, current_position: &mut u32, last_note_start: &mut u32) {
    let (is_chord, duration, is_grace) = match &note.content.info {
        NoteType::Normal(normal) => (normal.chord.is_some(), *normal.duration.content, false),
        NoteType::Cue(cue) => (cue.chord.is_some(), *cue.duration.content, false),
        NoteType::Grace(grace) => {
            let is_chord = match &grace.info {
                GraceType::Normal(n) => n.chord.is_some(),
                GraceType::Cue(c) => c.chord.is_some(),
            };
            (is_chord, 0, true)
        }
    };

    if !is_chord {
        *last_note_start = *current_position;
        if !is_grace {
            *current_position = current_position.saturating_add(duration);
        }
    }
}

enum ScoreElement<'a> {
    Attributes(&'a musicxml::elements::Attributes),
    Direction(&'a musicxml::elements::Direction),
    Sound(&'a musicxml::elements::Sound),
    Backup(&'a musicxml::elements::Backup),
    Forward(&'a musicxml::elements::Forward),
    Barline(&'a musicxml::elements::Barline),
    Note(&'a musicxml::elements::Note),
}

impl<'a> ScoreElement<'a> {
    fn from_measure_element(elem: &'a MeasureElement) -> Option<Self> {
        match elem {
            MeasureElement::Attributes(a) => Some(ScoreElement::Attributes(a)),
            MeasureElement::Direction(d) => Some(ScoreElement::Direction(d)),
            MeasureElement::Sound(s) => Some(ScoreElement::Sound(s)),
            MeasureElement::Backup(b) => Some(ScoreElement::Backup(b)),
            MeasureElement::Forward(f) => Some(ScoreElement::Forward(f)),
            MeasureElement::Barline(b) => Some(ScoreElement::Barline(b)),
            MeasureElement::Note(n) => Some(ScoreElement::Note(n)),
            _ => None,
        }
    }

    fn from_part_element(elem: &'a PartElement) -> Option<Self> {
        match elem {
            PartElement::Attributes(a) => Some(ScoreElement::Attributes(a)),
            PartElement::Direction(d) => Some(ScoreElement::Direction(d)),
            PartElement::Sound(s) => Some(ScoreElement::Sound(s)),
            PartElement::Backup(b) => Some(ScoreElement::Backup(b)),
            PartElement::Forward(f) => Some(ScoreElement::Forward(f)),
            PartElement::Barline(b) => Some(ScoreElement::Barline(b)),
            PartElement::Note(n) => Some(ScoreElement::Note(n)),
            _ => None,
        }
    }
}

fn process_element<'a>(
    elem: ScoreElement<'a>,
    current_position: &mut u32,
    last_note_start: &mut u32,
    state: &mut ParserState,
    part_idx: u32,
    target_part_indices: Option<&[u32]>,
    inline_attrs: &mut Vec<InlineMeasureAttributes>,
    repeats: &mut Vec<(u32, RepeatVariant)>,
    raw_notes: &mut Vec<(u32, StartNote, EndNote)>,
    rests: &mut Vec<RestInfo>,
) {
    match elem {
        ScoreElement::Attributes(attrs) => {
            handle_attributes(attrs, *current_position, state, inline_attrs);
        }
        ScoreElement::Direction(dir) => {
            handle_direction(dir, *current_position, state, inline_attrs, repeats);
        }
        ScoreElement::Sound(sound) => {
            handle_sound(sound, *current_position, state, inline_attrs, repeats);
        }
        ScoreElement::Backup(backup) => {
            let dur = *backup.content.duration.content;
            *current_position = current_position.saturating_sub(dur);
            *last_note_start = *current_position;
        }
        ScoreElement::Forward(forward) => {
            let dur = *forward.content.duration.content;
            *current_position = current_position.saturating_add(dur);
            *last_note_start = *current_position;
        }
        ScoreElement::Barline(barline) => {
            handle_barline(barline, *current_position, repeats);
        }
        ScoreElement::Note(note) => {
            let should_include = match target_part_indices {
                Some(indices) => indices.contains(&part_idx),
                None => true,
            };
            if should_include {
                handle_note(
                    note,
                    current_position,
                    last_note_start,
                    state,
                    part_idx,
                    raw_notes,
                    rests,
                );
            } else {
                advance_note_position(note, current_position, last_note_start);
            }
        }
    }
}
