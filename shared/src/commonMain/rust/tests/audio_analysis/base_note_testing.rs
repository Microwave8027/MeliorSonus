use crate::audio_analysis::preprocessing_mxl::mxl_metadata::{MxlMetaData, PartListType};
use crate::audio_analysis::preprocessing_mxl::preprocess::MusicXmlParser;
use musicxml::read_score_timewise;
use std::path::Path;
use std::sync::Arc;

#[test]
fn test_parse_fur_elise_timewise_xml() {
    let path =
        "src/commonMain/rust/tests/audio_analysis/testing_assets/fur_elise_mxl/fur_elise.xml";
    let score = read_score_timewise(path).expect("failed to read xml");
    let meta = MxlMetaData::from(&score.content);
    println!("XML Meta: {:?}", meta);
}

#[test]
fn test_parse_fur_elise_timewise_mxl() {
    let path =
        "src/commonMain/rust/tests/audio_analysis/testing_assets/fur_elise_mxl/fur_elise.mxl";
    let score = read_score_timewise(path).expect("failed to read mxl");
    let meta = MxlMetaData::from(&score.content);
    println!("MXL Meta: {:?}", meta);
    println!("Measures count: {}", score.content.measure.len());
    if let Some(first_measure) = score.content.measure.first() {
        println!(
            "First measure elements count: {}",
            first_measure.content.len()
        );
        for elem in &first_measure.content {
            match elem {
                musicxml::elements::MeasureElement::Part(p) => {
                    println!(
                        "  Part ID: {}, elements: {}",
                        p.attributes.id.0,
                        p.content.len()
                    );
                    for pe in &p.content {
                        match pe {
                            musicxml::elements::PartElement::Note(n) => {
                                let duration = match &n.content.info {
                                    musicxml::elements::NoteType::Normal(normal) => {
                                        Some(*normal.duration.content)
                                    }
                                    musicxml::elements::NoteType::Cue(cue) => {
                                        Some(*cue.duration.content)
                                    }
                                    musicxml::elements::NoteType::Grace(_) => None,
                                };
                                println!(
                                    "    Note: type={:?}, duration={:?}, staff={:?}",
                                    n.content.r#type, duration, n.content.staff
                                );
                            }
                            musicxml::elements::PartElement::Attributes(_) => {
                                println!("    Attributes element");
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
