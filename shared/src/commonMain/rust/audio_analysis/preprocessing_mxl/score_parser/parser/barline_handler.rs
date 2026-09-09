use crate::audio_analysis::score_parser::repeats::{
    BarlineLocation, EndingType, JumpKind, RepeatEnd, RepeatEnding, RepeatJump, RepeatStart,
    RepeatVariant,
};
use musicxml::datatypes::{BackwardForward, RightLeftMiddle, StartStopDiscontinue, YesNo};
use musicxml::elements::Barline;

pub fn handle_barline(
    barline: &Barline,
    current_position: u32,
    repeats: &mut Vec<(u32, RepeatVariant)>,
) {
    let location = match barline.attributes.location {
        Some(RightLeftMiddle::Left) => BarlineLocation::Left,
        Some(RightLeftMiddle::Right) => BarlineLocation::Right,
        Some(RightLeftMiddle::Middle) => BarlineLocation::Middle,
        None => BarlineLocation::Right,
    };

    if let Some(repeat) = &barline.content.repeat {
        match repeat.attributes.direction {
            BackwardForward::Forward => {
                repeats.push((
                    current_position,
                    RepeatVariant::Start(RepeatStart {
                        location,
                        winged: repeat.attributes.winged.is_some().then_some(true),
                    }),
                ));
            }
            BackwardForward::Backward => {
                let times = repeat.attributes.times.as_ref().map(|t| t.0).unwrap_or(2);
                let after_jump = repeat.attributes.after_jump == Some(YesNo::Yes);
                repeats.push((
                    current_position,
                    RepeatVariant::End(RepeatEnd {
                        location,
                        times,
                        after_jump,
                        winged: repeat.attributes.winged.is_some().then_some(true),
                    }),
                ));
            }
        }
    }

    if let Some(ending) = &barline.content.ending {
        let ending_type = match ending.attributes.r#type {
            StartStopDiscontinue::Start => EndingType::Start,
            StartStopDiscontinue::Stop => EndingType::Stop,
            StartStopDiscontinue::Discontinue => EndingType::Discontinue,
        };
        let text = if ending.content.trim().is_empty() {
            None
        } else {
            Some(ending.content.trim().to_string())
        };
        repeats.push((
            current_position,
            RepeatVariant::Ending(RepeatEnding {
                number: ending.attributes.number.0.clone(),
                ending_type,
                text,
            }),
        ));
    }

    if barline.content.segno.is_some() {
        repeats.push((
            current_position,
            RepeatVariant::Jump(RepeatJump {
                kind: JumpKind::Segno,
                text: None,
            }),
        ));
    }

    if barline.content.coda.is_some() {
        repeats.push((
            current_position,
            RepeatVariant::Jump(RepeatJump {
                kind: JumpKind::Coda,
                text: None,
            }),
        ));
    }
}
