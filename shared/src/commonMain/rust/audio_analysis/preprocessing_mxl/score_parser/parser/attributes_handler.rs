use super::state::ParserState;
use crate::audio_analysis::score_parser::attributes::{
    ClefSign, InlineAttributeKind, InlineMeasureAttributes,
};
use musicxml::datatypes;
use musicxml::elements::KeyContents;

pub fn handle_attributes(
    attrs: &musicxml::elements::Attributes,
    current_position: u32,
    state: &mut ParserState,
    inline_attrs: &mut Vec<InlineMeasureAttributes>,
) {
    if let Some(div) = &attrs.content.divisions {
        state.current_divisions = *div.content;
    }

    for transpose in &attrs.content.transpose {
        let chromatic = *transpose.content.chromatic.content as i8;
        let octave = transpose
            .content
            .octave_change
            .as_ref()
            .map(|o| o.content)
            .unwrap_or(0);
        state.current_transpose_semitones = chromatic.saturating_add(octave.saturating_mul(12));
    }

    for clef in &attrs.content.clef {
        let sign = match clef.content.sign.content {
            datatypes::ClefSign::G => ClefSign::G,
            datatypes::ClefSign::F => ClefSign::F,
            datatypes::ClefSign::C => ClefSign::C,
            datatypes::ClefSign::Percussion => ClefSign::Percussion,
            datatypes::ClefSign::TAB => ClefSign::TAB,
            _ => ClefSign::Other,
        };
        let line = clef.content.line.as_ref().map(|l| l.content.0 as i8);
        let staff = clef.attributes.number.as_ref().map(|s| s.0 as u32);
        inline_attrs.push(InlineMeasureAttributes {
            division_offset: current_position,
            kind: InlineAttributeKind::Clef { sign, line, staff },
        });
    }

    for key in &attrs.content.key {
        let fifths = match &key.content {
            KeyContents::Explicit(exp) => exp.fifths.content.0,
            KeyContents::Relative(_) => 0,
        };
        inline_attrs.push(InlineMeasureAttributes {
            division_offset: current_position,
            kind: InlineAttributeKind::KeySignature { fifths },
        });
    }

    for time in &attrs.content.time {
        for tb in &time.content.beats {
            let beats = tb.beats.content.parse().unwrap_or(4);
            let beat_type = tb.beat_type.content.parse().unwrap_or(4);
            inline_attrs.push(InlineMeasureAttributes {
                division_offset: current_position,
                kind: InlineAttributeKind::TimeSignature { beats, beat_type },
            });
        }
    }
}
