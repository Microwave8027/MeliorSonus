use super::state::ParserState;
use crate::audio_analysis::score_parser::attributes::RestInfo;
use crate::audio_analysis::score_parser::notes::{
    DynamicLevel, EndNote, NoteArticulation, Octave, Pitch, StartNote, TieType,
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

    if let Some(audible) = audible {
        match audible {
            AudibleType::Pitch(p) => {
                let step = &p.content.step.content;
                let oct = p.content.octave.content.0;
                let alter = p.content.alter.as_ref().map(|a| a.content.0).unwrap_or(0);

                let base_step_semitone = step_to_semitones(step);
                let midi = ((oct as i32 + 1) * 12 + base_step_semitone as i32 + alter as i32)
                    .clamp(0, 127) as u8;
                let pitch = Pitch::from_midi(midi);
                let octave = Octave::from_midi(midi);

                let dynamic = state.current_dynamic.unwrap_or(DynamicLevel::MezzoForte);
                let velocity = if let Some(dyn_attr) = &note.attributes.dynamics {
                    ((dyn_attr.0 / 100.0) * 90.0).clamp(1.0, 127.0) as u8
                } else {
                    dynamic.default_velocity()
                };

                let articulation = extract_articulation(note);
                let tie = extract_tie(note);

                let start_note = StartNote {
                    pitch,
                    octave,
                    alter: alter as i8,
                    tonality_offset: alter as i8,
                    velocity,
                    dynamic,
                    start_division: start_div,
                    duration_divisions: duration,
                    midi_note: midi,
                    voice,
                    staff,
                    is_grace,
                };

                let end_note = EndNote {
                    pitch,
                    octave,
                    alter: alter as i8,
                    tonality_offset: alter as i8,
                    duration_divisions: duration,
                    start_division: start_div,
                    end_division: start_div.saturating_add(duration),
                    articulation,
                    dynamic,
                    velocity,
                    tie,
                    midi_note: midi,
                    voice,
                    staff,
                    is_grace,
                };

                raw_notes.push((start_div, start_note, end_note));
            }
            AudibleType::Rest(_) => {
                rests.push(RestInfo {
                    start_division: start_div,
                    duration,
                    voice,
                    staff,
                });
            }
            AudibleType::Unpitched(_) => {
                let dynamic = state.current_dynamic.unwrap_or(DynamicLevel::MezzoForte);
                let velocity = dynamic.default_velocity();
                let articulation = extract_articulation(note);
                let tie = extract_tie(note);

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
