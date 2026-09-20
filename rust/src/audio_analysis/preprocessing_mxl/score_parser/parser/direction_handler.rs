use super::sound_handler::handle_sound;
use super::state::ParserState;
use crate::audio_analysis::score_parser::attributes::{
    InlineAttributeKind, InlineMeasureAttributes, PedalType, TempoChangeKind, WedgeType,
};
use crate::audio_analysis::score_parser::notes::DynamicLevel;
use crate::audio_analysis::score_parser::repeats::{JumpKind, RepeatJump, RepeatVariant};
use musicxml::datatypes;
use musicxml::elements::{BeatEquation, Direction, DirectionTypeContents, DynamicsType, MetronomeContents};

pub fn handle_direction(
    direction: &Direction,
    current_position: u32,
    state: &mut ParserState,
    inline_attrs: &mut Vec<InlineMeasureAttributes>,
    repeats: &mut Vec<(u32, RepeatVariant)>,
) {
    let offset = direction
        .content
        .offset
        .as_ref()
        .map(|o| o.content.0 as u32)
        .unwrap_or(0);
    let division_offset = current_position.saturating_add(offset);

    if let Some(sound) = &direction.content.sound {
        handle_sound(sound, division_offset, state, inline_attrs, repeats);
    }

    for dt in &direction.content.direction_type {
        match &dt.content {
            DirectionTypeContents::Words(words_vec) => {
                for word in words_vec {
                    let text = word.content.trim().to_string();
                    let lower = text.to_lowercase();
                    if lower.contains("rit") || lower.contains("rall") {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::TempoChange {
                                bpm: None,
                                kind: TempoChangeKind::Ritardando,
                                text: text.clone(),
                            },
                        });
                    } else if lower.contains("accel") {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::TempoChange {
                                bpm: None,
                                kind: TempoChangeKind::Accelerando,
                                text: text.clone(),
                            },
                        });
                    } else if lower.contains("a tempo")
                        || lower.contains("tempo i")
                        || lower.contains("tempo primo")
                    {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::TempoChange {
                                bpm: state.current_bpm,
                                kind: TempoChangeKind::ATempo,
                                text: text.clone(),
                            },
                        });
                    } else if lower == "cresc." || lower.contains("crescendo") {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::Wedge {
                                wedge_type: WedgeType::Crescendo,
                            },
                        });
                    } else if lower == "dim."
                        || lower.contains("diminuendo")
                        || lower.contains("decresc")
                    {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::Wedge {
                                wedge_type: WedgeType::Diminuendo,
                            },
                        });
                    } else if lower == "d.c." || lower.contains("da capo") {
                        repeats.push((
                            division_offset,
                            RepeatVariant::Jump(RepeatJump {
                                kind: JumpKind::DaCapo,
                                text: Some(text.clone()),
                            }),
                        ));
                    } else if lower == "d.s." || lower.contains("dal segno") {
                        repeats.push((
                            division_offset,
                            RepeatVariant::Jump(RepeatJump {
                                kind: JumpKind::DalSegno,
                                text: Some(text.clone()),
                            }),
                        ));
                    } else if lower == "fine" {
                        repeats.push((
                            division_offset,
                            RepeatVariant::Jump(RepeatJump {
                                kind: JumpKind::Fine,
                                text: Some(text.clone()),
                            }),
                        ));
                    } else if lower.contains("to coda") {
                        repeats.push((
                            division_offset,
                            RepeatVariant::Jump(RepeatJump {
                                kind: JumpKind::ToCoda,
                                text: Some(text.clone()),
                            }),
                        ));
                    } else if let Some(dyn_level) = dynamic_from_word(&lower) {
                        state.current_dynamic = Some(dyn_level);
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::Dynamic {
                                level: dyn_level,
                                text: Some(text.clone()),
                            },
                        });
                    } else {
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::DirectionWords(text.clone()),
                        });
                    }
                }
            }
            DirectionTypeContents::Dynamics(dynamics_vec) => {
                for dyn_elem in dynamics_vec {
                    for dt_elem in &dyn_elem.content {
                        let level = dynamic_from_dynamics_type(dt_elem);
                        state.current_dynamic = Some(level);
                        inline_attrs.push(InlineMeasureAttributes {
                            division_offset,
                            kind: InlineAttributeKind::Dynamic { level, text: None },
                        });
                    }
                }
            }
            DirectionTypeContents::Wedge(wedge) => {
                let wedge_type = match wedge.attributes.r#type {
                    datatypes::WedgeType::Crescendo => WedgeType::Crescendo,
                    datatypes::WedgeType::Diminuendo => WedgeType::Diminuendo,
                    datatypes::WedgeType::Stop => WedgeType::Stop,
                    _ => WedgeType::Stop,
                };
                inline_attrs.push(InlineMeasureAttributes {
                    division_offset,
                    kind: InlineAttributeKind::Wedge { wedge_type },
                });
            }
            DirectionTypeContents::Metronome(metronome) => {
                if let MetronomeContents::BeatBased(beat_based) = &metronome.content {
                    if let BeatEquation::BPM(per_minute) = &beat_based.equals {
                        if let Ok(bpm_val) = per_minute.content.trim().parse::<f64>() {
                            let bpm_u32 = bpm_val.round() as u32;
                            state.current_bpm = Some(bpm_u32);
                            inline_attrs.push(InlineMeasureAttributes {
                                division_offset,
                                kind: InlineAttributeKind::TempoChange {
                                    bpm: Some(bpm_u32),
                                    kind: TempoChangeKind::BpmChange,
                                    text: format!("{} BPM", bpm_u32),
                                },
                            });
                        }
                    }
                }
            }
            DirectionTypeContents::Pedal(pedal) => {
                let p_type = match pedal.attributes.r#type {
                    datatypes::PedalType::Start
                    | datatypes::PedalType::Continue
                    | datatypes::PedalType::Resume => {
                        state.is_pedal_active = true;
                        PedalType::Start
                    }
                    datatypes::PedalType::Stop | datatypes::PedalType::Discontinue => {
                        state.is_pedal_active = false;
                        PedalType::Stop
                    }
                    datatypes::PedalType::Sostenuto => {
                        state.is_pedal_active = true;
                        PedalType::Sostenuto
                    }
                    datatypes::PedalType::Change => {
                        state.is_pedal_active = true;
                        PedalType::Change
                    }
                };
                inline_attrs.push(InlineMeasureAttributes {
                    division_offset,
                    kind: InlineAttributeKind::Pedal {
                        pedal_type: p_type,
                    },
                });
            }
            DirectionTypeContents::OctaveShift(shift) => {
                let size = shift.attributes.size.as_ref().map(|s| s.0 as i8).unwrap_or(8);
                let semitones = match size {
                    15 => 24,
                    _ => 12,
                };
                let shift_val = match shift.attributes.r#type {
                    datatypes::UpDownStopContinue::Down => semitones,
                    datatypes::UpDownStopContinue::Up => -semitones,
                    datatypes::UpDownStopContinue::Stop => 0,
                    datatypes::UpDownStopContinue::Continue => state.current_octave_shift,
                };
                state.current_octave_shift = shift_val;
                inline_attrs.push(InlineMeasureAttributes {
                    division_offset,
                    kind: InlineAttributeKind::OctaveShift {
                        semitones: shift_val,
                    },
                });
            }
            DirectionTypeContents::Segno(_) => {
                repeats.push((
                    division_offset,
                    RepeatVariant::Jump(RepeatJump {
                        kind: JumpKind::Segno,
                        text: None,
                    }),
                ));
            }
            DirectionTypeContents::Coda(_) => {
                repeats.push((
                    division_offset,
                    RepeatVariant::Jump(RepeatJump {
                        kind: JumpKind::Coda,
                        text: None,
                    }),
                ));
            }
            _ => {}
        }
    }
}

pub fn dynamic_from_dynamics_type(dt: &DynamicsType) -> DynamicLevel {
    match dt {
        DynamicsType::Ppp(_)
        | DynamicsType::Pppp(_)
        | DynamicsType::Ppppp(_)
        | DynamicsType::Pppppp(_) => DynamicLevel::Pianississimo,
        DynamicsType::Pp(_) => DynamicLevel::Pianissimo,
        DynamicsType::P(_) => DynamicLevel::Piano,
        DynamicsType::Mp(_) => DynamicLevel::MezzoPiano,
        DynamicsType::Mf(_) => DynamicLevel::MezzoForte,
        DynamicsType::F(_) => DynamicLevel::Forte,
        DynamicsType::Ff(_) => DynamicLevel::Fortissimo,
        DynamicsType::Fff(_)
        | DynamicsType::Ffff(_)
        | DynamicsType::Fffff(_)
        | DynamicsType::Ffffff(_) => DynamicLevel::Fortississimo,
        DynamicsType::Sf(_)
        | DynamicsType::Sfz(_)
        | DynamicsType::Sffz(_)
        | DynamicsType::Fz(_)
        | DynamicsType::Rf(_)
        | DynamicsType::Rfz(_) => DynamicLevel::Fortissimo,
        DynamicsType::Fp(_)
        | DynamicsType::Sfp(_)
        | DynamicsType::Sfzp(_) => DynamicLevel::Forte,
        _ => DynamicLevel::Other,
    }
}

pub fn dynamic_from_word(word: &str) -> Option<DynamicLevel> {
    match word {
        "ppp" | "pianississimo" => Some(DynamicLevel::Pianississimo),
        "pp" | "pianissimo" => Some(DynamicLevel::Pianissimo),
        "p" | "piano" => Some(DynamicLevel::Piano),
        "mp" | "mezzo-piano" | "mezzopiano" => Some(DynamicLevel::MezzoPiano),
        "mf" | "mezzo-forte" | "mezzoforte" => Some(DynamicLevel::MezzoForte),
        "f" | "forte" => Some(DynamicLevel::Forte),
        "ff" | "fortissimo" => Some(DynamicLevel::Fortissimo),
        "fff" | "fortississimo" => Some(DynamicLevel::Fortississimo),
        "sf" | "sfz" | "sffz" | "fz" | "rf" | "rfz" => Some(DynamicLevel::Fortissimo),
        "fp" | "sfp" => Some(DynamicLevel::Forte),
        _ => None,
    }
}
