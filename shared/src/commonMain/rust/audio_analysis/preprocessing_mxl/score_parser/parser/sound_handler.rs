use super::state::ParserState;
use crate::audio_analysis::score_parser::attributes::{
    InlineAttributeKind, InlineMeasureAttributes, TempoChangeKind,
};
use crate::audio_analysis::score_parser::notes::DynamicLevel;
use crate::audio_analysis::score_parser::repeats::{JumpKind, RepeatJump, RepeatVariant};
use musicxml::datatypes::YesNo;
use musicxml::elements::Sound;

pub fn handle_sound(
    sound: &Sound,
    current_position: u32,
    state: &mut ParserState,
    inline_attrs: &mut Vec<InlineMeasureAttributes>,
    repeats: &mut Vec<(u32, RepeatVariant)>,
) {
    if let Some(tempo) = &sound.attributes.tempo {
        let bpm = tempo.0 as u32;
        state.current_bpm = Some(bpm);
        inline_attrs.push(InlineMeasureAttributes {
            division_offset: current_position,
            kind: InlineAttributeKind::TempoChange {
                bpm: Some(bpm),
                kind: TempoChangeKind::BpmChange,
                text: format!("{} BPM", bpm),
            },
        });
    }
    if let Some(dyn_val) = &sound.attributes.dynamics {
        let dyn_pct = dyn_val.0;
        let level = dynamic_from_percentage(dyn_pct);
        state.current_dynamic = Some(level);
        inline_attrs.push(InlineMeasureAttributes {
            division_offset: current_position,
            kind: InlineAttributeKind::Dynamic { level, text: None },
        });
    }
    if sound.attributes.dacapo == Some(YesNo::Yes) {
        repeats.push((
            current_position,
            RepeatVariant::Jump(RepeatJump {
                kind: JumpKind::DaCapo,
                text: None,
            }),
        ));
    }
    if sound.attributes.segno.is_some() {
        repeats.push((
            current_position,
            RepeatVariant::Jump(RepeatJump {
                kind: JumpKind::Segno,
                text: None,
            }),
        ));
    }
    if sound.attributes.coda.is_some() {
        repeats.push((
            current_position,
            RepeatVariant::Jump(RepeatJump {
                kind: JumpKind::Coda,
                text: None,
            }),
        ));
    }
}

pub fn dynamic_from_percentage(pct: f64) -> DynamicLevel {
    if pct < 35.0 {
        DynamicLevel::Pianississimo
    } else if pct < 50.0 {
        DynamicLevel::Pianissimo
    } else if pct < 65.0 {
        DynamicLevel::Piano
    } else if pct < 80.0 {
        DynamicLevel::MezzoPiano
    } else if pct < 95.0 {
        DynamicLevel::MezzoForte
    } else if pct < 110.0 {
        DynamicLevel::Forte
    } else if pct < 120.0 {
        DynamicLevel::Fortissimo
    } else {
        DynamicLevel::Fortississimo
    }
}
