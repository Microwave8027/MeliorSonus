use crate::audio_analysis::mxl_metadata::MxlMetaData;
use crate::audio_analysis::preprocessing_mxl::preprocess::MusicXmlParser;
use crate::audio_analysis::score_parser::{
    DynamicLevel, InlineAttributeKind, NoteCluster, Octave, Pitch, RepeatVariant,
    SequentialMusicScore,
};
use musicxml::read_score_timewise;
use std::path::Path;

const FUR_ELISE_MXL_PATH: &str =
    "src/commonMain/rust/tests/audio_analysis/testing_assets/fur_elise_mxl/fur_elise.mxl";

#[test]
fn test_fur_elise_score_parser_mxl() {
    // 1. Read MXL file with read_score_timewise
    let score = read_score_timewise(FUR_ELISE_MXL_PATH).expect("Failed to read fur_elise.mxl");
    let meta = MxlMetaData::from(&score.content);

    assert_eq!(meta.title, "Für Elise");
    assert_eq!(meta.composer, "Ludwig van Beethoven");

    // 2. Parse into SequentialMusicScore using the metadata part_list_type
    let parsed_score = SequentialMusicScore::from(&score.content.measure, &meta.part_list_type);

    // Verify measures count
    assert_eq!(parsed_score.total_measures(), 106);

    // 3. Flattened note access for comparison against audio processing pipeline
    let all_start_notes = parsed_score.all_start_notes();
    let all_end_notes = parsed_score.all_end_notes();

    assert!(!all_start_notes.is_empty(), "Score should contain notes");
    assert_eq!(
        all_start_notes.len(),
        all_end_notes.len(),
        "Every start note must have a corresponding end note"
    );

    // 4. Verify famous opening motif of Für Elise: E5, D#5, E5, D#5, E5, B4, D5, C5, A4
    assert_eq!(all_start_notes[0].pitch, Pitch::E);
    assert_eq!(all_start_notes[0].octave, Octave::O5);
    assert_eq!(all_start_notes[0].midi_note, 76);

    assert_eq!(all_start_notes[1].pitch, Pitch::DsEf);
    assert_eq!(all_start_notes[1].octave, Octave::O5);
    assert_eq!(all_start_notes[1].midi_note, 75);

    // Corresponding end note checks
    assert_eq!(all_end_notes[0].pitch, Pitch::E);
    assert_eq!(all_end_notes[0].octave, Octave::O5);
    assert!(all_end_notes[0].duration_divisions > 0);
    assert_eq!(
        all_end_notes[0].end_division,
        all_end_notes[0].start_division + all_end_notes[0].duration_divisions
    );

    // 5. Test conversion directly to audio_processing types
    let audio_pitch: crate::audio_processing::instruments::notes::Pitch =
        all_start_notes[0].pitch.into();
    assert_eq!(
        audio_pitch,
        crate::audio_processing::instruments::notes::Pitch::E
    );

    let audio_octave: crate::audio_processing::instruments::notes::Octave =
        all_start_notes[0].octave.into();
    assert_eq!(
        audio_octave,
        crate::audio_processing::instruments::notes::Octave::O5
    );

    let audio_dynamic: crate::audio_processing::instruments::notes::DynamicLevel =
        all_start_notes[0].dynamic.into();
    assert!(matches!(
        audio_dynamic,
        crate::audio_processing::instruments::notes::DynamicLevel::Pianissimo
            | crate::audio_processing::instruments::notes::DynamicLevel::Piano
            | crate::audio_processing::instruments::notes::DynamicLevel::MezzoPiano
            | crate::audio_processing::instruments::notes::DynamicLevel::MezzoForte
    ));

    // 6. Test measure-level cluster access and chord detection
    let mut chord_count = 0;
    for measure in &parsed_score.notes {
        for cluster in measure.notes() {
            if cluster.is_chord() {
                chord_count += 1;
                assert!(cluster.start_notes.len() >= 2);
                assert_eq!(cluster.start_notes.len(), cluster.end_notes.len());
            }
        }
    }
    assert!(chord_count > 0, "Für Elise should contain harmony chords");
}

#[test]
fn test_fur_elise_repeats_and_inline_attributes() {
    let score = read_score_timewise(FUR_ELISE_MXL_PATH).expect("Failed to read fur_elise.mxl");
    let meta = MxlMetaData::from(&score.content);
    let parsed_score = SequentialMusicScore::from(&score.content.measure, &meta.part_list_type);

    let mut found_repeat = false;
    let mut found_inline_attr = false;

    for measure in &parsed_score.notes {
        for repeat in measure.repeats() {
            found_repeat = true;
            match repeat {
                RepeatVariant::Start(s) => {
                    assert!(matches!(
                        s.location,
                        crate::audio_analysis::score_parser::BarlineLocation::Left
                            | crate::audio_analysis::score_parser::BarlineLocation::Right
                    ));
                }
                RepeatVariant::End(e) => {
                    assert!(e.times >= 1);
                }
                _ => {}
            }
        }

        for attr in measure.inline_attributes() {
            found_inline_attr = true;
            match &attr.kind {
                InlineAttributeKind::Dynamic { level, .. } => {
                    assert_ne!(*level, DynamicLevel::Other);
                }
                InlineAttributeKind::TempoChange { kind: _, text, .. } => {
                    assert!(!text.is_empty());
                }
                InlineAttributeKind::Clef { .. } => {}
                InlineAttributeKind::KeySignature { .. } => {}
                InlineAttributeKind::TimeSignature { .. } => {}
                _ => {}
            }
        }
    }

    assert!(found_repeat, "Für Elise should contain repeats");
    assert!(
        found_inline_attr,
        "Für Elise should contain inline attributes (dynamics/clefs)"
    );
}

#[test]
fn test_music_xml_parser_invalid_paths() {
    let invalid_mxl: &'static Path = Path::new("non_existent_file_12345.mxl");
    let target: &'static Path = Path::new("target.bin");
    let meta: &'static Path = Path::new("meta.bin");

    let parser = MusicXmlParser::new(invalid_mxl, target, meta);
    let result = parser.parse_mxl();
    assert!(result.is_err(), "parse_mxl should fail on non-existent path");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "Error message should mention 'does not exist', but got: {}",
        err_msg
    );
}
