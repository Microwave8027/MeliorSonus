use super::part_list_type::PartListType;
use musicxml::elements::*;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct MxlMetaData {
    pub title: String,
    pub composer: String,
    pub movement: Option<String>,
    pub instruments: Vec<String>,
    pub song_length_bars: u32,
    pub part_list_type: PartListType,
}

impl MxlMetaData {
    pub fn from(contents: &ScoreTimewiseContents) -> Self {
        parse_mxl_metadata(contents)
    }
}

pub fn parse_mxl_metadata(contents: &ScoreTimewiseContents) -> MxlMetaData {
    let movement_title = contents.movement_title.as_ref().map(|m| m.content.clone());
        let movement_number = contents.movement_number.as_ref().map(|m| m.content.clone());
        let work_title = contents
            .work
            .as_ref()
            .and_then(|w| w.content.work_title.as_ref().map(|t| t.content.clone()));

        let (title, movement) = match (work_title, movement_title) {
            (Some(w), Some(m)) => (w, Some(m)),
            (Some(w), None) => (w, movement_number),
            (None, Some(m)) => (m, movement_number),
            (None, None) => {
                let credit_title = contents.credit.iter().find_map(|c| {
                    let is_title = c
                        .content
                        .credit_type
                        .iter()
                        .any(|ct| ct.content.eq_ignore_ascii_case("title"));
                    if is_title {
                        if let CreditSubcontents::Text(ref text) = c.content.credit {
                            return text.credit_words.as_ref().map(|w| w.content.clone());
                        }
                    }
                    None
                });
                (
                    credit_title.unwrap_or_else(|| "Unknown Title".to_string()),
                    movement_number,
                )
            }
        };

        let composer = contents
            .identification
            .as_ref()
            .and_then(|id| {
                id.content
                    .creator
                    .iter()
                    .find(|c| {
                        c.attributes
                            .r#type
                            .as_ref()
                            .map_or(false, |t| t.0.eq_ignore_ascii_case("composer"))
                    })
                    .map(|c| c.content.clone())
                    .or_else(|| id.content.creator.first().map(|c| c.content.clone()))
            })
            .or_else(|| {
                contents.credit.iter().find_map(|c| {
                    let is_composer = c
                        .content
                        .credit_type
                        .iter()
                        .any(|ct| ct.content.eq_ignore_ascii_case("composer"));
                    if is_composer {
                        if let CreditSubcontents::Text(ref text) = c.content.credit {
                            return text.credit_words.as_ref().map(|w| w.content.clone());
                        }
                    }
                    None
                })
            })
            .unwrap_or_else(|| "Unknown Composer".to_string());

        let mut current_group: Option<String> = None;
        let mut instruments: Vec<String> = Vec::new();

        for elem in &contents.part_list.content.content {
            match elem {
                PartListElement::PartGroup(group) => match group.attributes.r#type {
                    musicxml::datatypes::StartStop::Start => {
                        current_group = group
                            .content
                            .group_name
                            .as_ref()
                            .map(|g| g.content.trim().to_string())
                            .filter(|g| !g.is_empty());
                    }
                    musicxml::datatypes::StartStop::Stop => {
                        current_group = None;
                    }
                },
                PartListElement::ScorePart(part) => {
                    let part_name = part.content.part_name.content.trim().to_string();
                    let full_name = match &current_group {
                        Some(group) => {
                            if part_name.is_empty() {
                                group.clone()
                            } else if part_name.to_lowercase().contains(&group.to_lowercase()) {
                                part_name
                            } else {
                                format!("{group} - {part_name}")
                            }
                        }
                        None => part_name,
                    };
                    instruments.push(full_name);
                }
            }
        }

        let part_list_type = if instruments.len() > 1 {
            PartListType::Parts(instruments.len() as u32)
        } else {
            let staves = contents
                .measure
                .iter()
                .find_map(|m| {
                    m.content.iter().find_map(|elem| match elem {
                        MeasureElement::Attributes(attrs) => {
                            attrs.content.staves.as_ref().map(|s| s.content.0)
                        }
                        MeasureElement::Part(part) => {
                            part.content.iter().find_map(|pe| match pe {
                                PartElement::Attributes(attrs) => {
                                    attrs.content.staves.as_ref().map(|s| s.content.0)
                                }
                                _ => None,
                            })
                        }
                        _ => None,
                    })
                })
                .unwrap_or(1);

            if staves > 1 {
                PartListType::Staves(staves)
            } else {
                PartListType::Parts(instruments.len().max(1) as u32)
            }
        };

        let song_length_bars = contents.measure.len() as u32;

        MxlMetaData {
            title,
            composer,
            movement,
            instruments,
            song_length_bars,
            part_list_type,
        }
    }
