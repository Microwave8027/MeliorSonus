use super::state::ParserState;
use crate::audio_analysis::score_parser::attributes::RestInfo;
use crate::audio_analysis::score_parser::notes::{
    DynamicLevel, EndNote, NoteArticulation, Octave, OrnamentKind, Pitch, StartNote, TieType,
};
use musicxml::datatypes::{StartStop, StartStopContinue, Step};
use musicxml::elements::{
    ArticulationsType, AudibleType, GraceType, NotationContentTypes, Note, NoteType,
};

pub fn handle_note(
    note: &Note,
    current_position: &mut u32,
    last_note_start: &mut u32,
    state: &mut ParserState,
    part_index: u32,
    raw_notes: &mut Vec<(u32, StartNote, EndNote)>,
    rests: &mut Vec<RestInfo>,
) {
    let (is_chord, duration, is_grace, audible) = match &note.content.info {
        NoteType::Normal(normal) => {
            let is_chord = normal.chord.is_some();
            let duration = *normal.duration.content;
            (is_chord, duration, false, Some(&normal.audible))
        }
        NoteType::Cue(cue) => {
            let is_chord = cue.chord.is_some();
            let duration = *cue.duration.content;
            (is_chord, duration, false, Some(&cue.audible))
        }
        NoteType::Grace(grace) => {
            let is_chord = match &grace.info {
                GraceType::Normal(n) => n.chord.is_some(),
                GraceType::Cue(c) => c.chord.is_some(),
            };
            let audible = match &grace.info {
                GraceType::Normal(n) => Some(&n.audible),
                GraceType::Cue(c) => Some(&c.audible),
            };
            (is_chord, 0, true, audible)
        }
    };

    let start_div = if is_chord {
        *last_note_start
    } else {
        *current_position
    };

    if !is_chord {
        *last_note_start = *current_position;
        if !is_grace {
            *current_position = current_position.saturating_add(duration);
        }
    }

    let voice: u32 = note
        .content
        .voice
        .as_ref()
        .and_then(|v| v.content.parse().ok())
        .unwrap_or(1);

    let staff: u32 = note
        .content
        .staff
        .as_ref()
        .map(|s| *s.content)
        .unwrap_or(1);

    let articulation = extract_articulation(note);
    let tie = extract_tie(note);
    let has_fermata = extract_fermata(note);
    let ornament = extract_ornament(note);

    if let Some(audible) = audible {
        match audible {
            AudibleType::Pitch(p) => {
                let step = &p.content.step.content;
                let oct = p.content.octave.content.0;
                let alter = p.content.alter.as_ref().map(|a| a.content.0).unwrap_or(0);

                let base_step_semitone = step_to_semitones(step);
                let sounding_midi = ((oct as i32 + 1) * 12
                    + base_step_semitone as i32
                    + alter as i32
                    + state.current_transpose_semitones as i32
                    + state.current_octave_shift as i32)
                    .clamp(0, 127) as u8;
                let pitch = midi_to_pitch(sounding_midi);
                let octave = midi_to_octave(sounding_midi);

                let dynamic = state.current_dynamic.unwrap_or(DynamicLevel::MezzoForte);
                let velocity = if let Some(dyn_attr) = &note.attributes.dynamics {
                    ((dyn_attr.0 / 100.0) * 90.0).clamp(1.0, 127.0) as u8
                } else {
                    default_velocity_for_dynamic(dynamic)
                };

                let start_note = StartNote {
                    pitch,
                    octave,
                    alter: alter as i8,
                    tonality_offset: 0,
                    velocity,
                    dynamic,
                    start_division: start_div,
                    duration_divisions: duration,
                    midi_note: sounding_midi,
                    voice,
                    staff,
                    is_grace,
                    tie,
                    part_index,
                    has_fermata,
                    ornament,
                };

                let end_note = EndNote {
                    pitch,
                    octave,
                    alter: alter as i8,
                    tonality_offset: 0,
                    duration_divisions: duration,
                    start_division: start_div,
                    end_division: start_div.saturating_add(duration),
                    articulation,
                    dynamic,
                    velocity,
                    tie,
                    midi_note: sounding_midi,
                    voice,
                    staff,
                    is_grace,
                    part_index,
                    has_fermata,
                    ornament,
                    is_pedaled: state.is_pedal_active,
                };

                raw_notes.push((start_div, start_note, end_note));
            }
            AudibleType::Rest(_) => {
                rests.push(RestInfo {
                    start_division: start_div,
                    duration,
                    voice,
                    staff,
                    part_index,
                });
            }
            AudibleType::Unpitched(_) => {
                let dynamic = state.current_dynamic.unwrap_or(DynamicLevel::MezzoForte);
                let velocity = default_velocity_for_dynamic(dynamic);

                let start_note = StartNote {
                    pitch: Pitch::None,
                    octave: Octave::OutOfRange,
                    alter: 0,
                    tonality_offset: 0,
                    velocity,
                    dynamic,
                    start_division: start_div,
                    duration_divisions: duration,
                    midi_note: 0,
                    voice,
                    staff,
                    is_grace,
                    tie,
                    part_index,
                    has_fermata,
                    ornament,
                };

                let end_note = EndNote {
                    pitch: Pitch::None,
                    octave: Octave::OutOfRange,
                    alter: 0,
                    tonality_offset: 0,
                    duration_divisions: duration,
                    start_division: start_div,
                    end_division: start_div.saturating_add(duration),
                    articulation,
                    dynamic,
                    velocity,
                    tie,
                    midi_note: 0,
                    voice,
                    staff,
                    is_grace,
                    part_index,
                    has_fermata,
                    ornament,
                    is_pedaled: state.is_pedal_active,
                };

                raw_notes.push((start_div, start_note, end_note));
            }
        }
    }
}

pub fn extract_articulation(note: &Note) -> NoteArticulation {
    for notation in &note.content.notations {
        for item in &notation.content.notations {
            match item {
                NotationContentTypes::Articulations(art) => {
                    for art_type in &art.content {
                        match art_type {
                            ArticulationsType::Staccato(_)
                            | ArticulationsType::Staccatissimo(_)
                            | ArticulationsType::Spiccato(_) => {
                                return NoteArticulation::Staccato;
                            }
                            ArticulationsType::Tenuto(_) => {
                                return NoteArticulation::Tenuto;
                            }
                            ArticulationsType::Accent(_) => {
                                return NoteArticulation::Accent;
                            }
                            ArticulationsType::StrongAccent(_) => {
                                return NoteArticulation::Marcato;
                            }
                            _ => {}
                        }
                    }
                }
                NotationContentTypes::Slur(_) => {
                    return NoteArticulation::Legato;
                }
                _ => {}
            }
        }
    }
    NoteArticulation::Normal
}

pub fn extract_tie(note: &Note) -> TieType {
    let mut has_start = false;
    let mut has_stop = false;

    if let NoteType::Normal(normal) = &note.content.info {
        for tie in &normal.tie {
            match tie.attributes.r#type {
                StartStop::Start => has_start = true,
                StartStop::Stop => has_stop = true,
            }
        }
    }

    for notation in &note.content.notations {
        for item in &notation.content.notations {
            if let NotationContentTypes::Tied(tied) = item {
                match tied.attributes.r#type {
                    StartStopContinue::Start => has_start = true,
                    StartStopContinue::Stop => has_stop = true,
                    StartStopContinue::Continue => {
                        has_start = true;
                        has_stop = true;
                    }
                }
            }
        }
    }

    match (has_start, has_stop) {
        (true, true) => TieType::Continue,
        (true, false) => TieType::Start,
        (false, true) => TieType::Stop,
        (false, false) => TieType::None,
    }
}

pub fn step_to_semitones(step: &Step) -> u8 {
    match step {
        Step::C => 0,
        Step::D => 2,
        Step::E => 4,
        Step::F => 5,
        Step::G => 7,
        Step::A => 9,
        Step::B => 11,
    }
}

pub fn extract_fermata(note: &Note) -> bool {
    for notation in &note.content.notations {
        for item in &notation.content.notations {
            if matches!(item, NotationContentTypes::Fermata(_)) {
                return true;
            }
        }
    }
    false
}

pub fn extract_ornament(note: &Note) -> Option<OrnamentKind> {
    let mut found_other = false;
    for notation in &note.content.notations {
        for item in &notation.content.notations {
            if let NotationContentTypes::Ornaments(orn) = item {
                for o in &orn.content.ornaments {
                    match o {
                        musicxml::elements::OrnamentType::TrillMark(_) => return Some(OrnamentKind::Trill),
                        musicxml::elements::OrnamentType::Turn(_) => return Some(OrnamentKind::Turn),
                        musicxml::elements::OrnamentType::InvertedTurn(_) => return Some(OrnamentKind::InvertedTurn),
                        musicxml::elements::OrnamentType::Mordent(_) => return Some(OrnamentKind::Mordent),
                        musicxml::elements::OrnamentType::InvertedMordent(_) => return Some(OrnamentKind::InvertedMordent),
                        musicxml::elements::OrnamentType::Tremolo(_) => return Some(OrnamentKind::Tremolo),
                        _ => {
                            found_other = true;
                        }
                    }
                }
            }
        }
    }
    if found_other {
        Some(OrnamentKind::Other)
    } else {
        None
    }
}

pub fn midi_to_pitch(midi: u8) -> Pitch {
    match midi % 12 {
        0 => Pitch::C,
        1 => Pitch::CsDf,
        2 => Pitch::D,
        3 => Pitch::DsEf,
        4 => Pitch::E,
        5 => Pitch::F,
        6 => Pitch::FsGf,
        7 => Pitch::G,
        8 => Pitch::GsAf,
        9 => Pitch::A,
        10 => Pitch::AsBf,
        11 => Pitch::B,
        _ => Pitch::None,
    }
}

pub fn midi_to_octave(midi: u8) -> Octave {
    match (midi as i8 / 12) - 1 {
        -1 => Octave::O_1,
        0 => Octave::O0,
        1 => Octave::O1,
        2 => Octave::O2,
        3 => Octave::O3,
        4 => Octave::O4,
        5 => Octave::O5,
        6 => Octave::O6,
        7 => Octave::O7,
        8 => Octave::O8,
        9 => Octave::O9,
        _ => Octave::OutOfRange,
    }
}

pub fn default_velocity_for_dynamic(dynamic: DynamicLevel) -> u8 {
    match dynamic {
        DynamicLevel::Pianississimo => 25,
        DynamicLevel::Pianissimo => 40,
        DynamicLevel::Piano => 55,
        DynamicLevel::MezzoPiano => 70,
        DynamicLevel::MezzoForte => 85,
        DynamicLevel::Forte => 100,
        DynamicLevel::Fortissimo => 115,
        DynamicLevel::Fortississimo => 127,
        DynamicLevel::Other => 80,
    }
}

