use crate::audio_analysis::mxl_metadata::MxlMetaData;
use crate::audio_analysis::preprocessing_mxl::preprocess::MusicXmlParser;
use crate::audio_analysis::score_parser::SequentialMusicScore;
use std::fs;
use std::path::Path;

const FUR_ELISE_MXL_PATH: &'static str =
    "src/commonMain/rust/tests/audio_analysis/testing_assets/fur_elise_mxl/fur_elise.mxl";
const RESULT_DIR: &'static str = "src/commonMain/rust/tests/audio_analysis/result";
const TARGET_MXL_PATH: &'static str =
    "src/commonMain/rust/tests/audio_analysis/result/fur_elise_score.rkyv";
const METADATA_PATH: &'static str =
    "src/commonMain/rust/tests/audio_analysis/result/fur_elise_meta.rkyv";

#[test]
fn test_serialzation_and_parsing() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Ensure the result directory exists
    fs::create_dir_all(RESULT_DIR)?;

    let mxl_path: &'static Path = Path::new(FUR_ELISE_MXL_PATH);
    let target_path: &'static Path = Path::new(TARGET_MXL_PATH);
    let metadata_path: &'static Path = Path::new(METADATA_PATH);

    // Clean up any existing result files from previous runs to ensure fresh test
    if target_path.exists() {
        let _ = fs::remove_file(target_path);
    }
    if metadata_path.exists() {
        let _ = fs::remove_file(metadata_path);
    }

    let parser = MusicXmlParser::new(mxl_path, target_path, metadata_path);

    // 2. Parse from original MXL asset
    let (original_meta, original_score) = parser.parse_mxl()?;
    assert_eq!(original_meta.title, "Für Elise");
    assert_eq!(original_meta.composer, "Ludwig van Beethoven");
    assert_eq!(original_score.total_measures(), 106);

    // 3. Store to custom target paths in the result folder
    parser.store_mxl(original_meta.clone(), original_score.clone())?;

    assert!(
        target_path.exists(),
        "Target score file should exist on disk"
    );
    assert!(metadata_path.exists(), "Metadata file should exist on disk");
    assert!(
        fs::metadata(target_path)?.len() > 0,
        "Target score file must not be empty"
    );
    assert!(
        fs::metadata(metadata_path)?.len() > 0,
        "Metadata file must not be empty"
    );

    // 4. Deserialize zero-copy via memory mapping
    let borrowed_score = parser.deserialize_music_score()?;
    let borrowed_meta = parser.deserialize_mxl_meta_data()?;

    // 5. Verify deserialized metadata matches original
    assert_eq!(borrowed_meta.title.as_str(), original_meta.title.as_str());
    assert_eq!(
        borrowed_meta.composer.as_str(),
        original_meta.composer.as_str()
    );
    assert_eq!(
        borrowed_meta.song_length_bars,
        original_meta.song_length_bars
    );
    assert_eq!(
        borrowed_meta.instruments.len(),
        original_meta.instruments.len()
    );

    // 6. Verify deserialized score measures and content match original
    assert_eq!(borrowed_score.notes.len(), original_score.total_measures());
    assert_eq!(borrowed_score.notes.len(), 106);

    // Verify first measure note content
    let first_measure = &borrowed_score.notes[0];
    assert!(!first_measure.content.is_empty());

    // 7. Also test `parse_and_store_mxl` convenience method
    // Remove files first and let parse_and_store_mxl re-create them
    fs::remove_file(target_path)?;
    fs::remove_file(metadata_path)?;
    assert!(!target_path.exists());
    assert!(!metadata_path.exists());

    parser.parse_and_store_mxl()?;
    assert!(
        target_path.exists(),
        "parse_and_store_mxl should write target file"
    );
    assert!(
        metadata_path.exists(),
        "parse_and_store_mxl should write metadata file"
    );

    let borrowed_score_2 = parser.deserialize_music_score()?;
    let borrowed_meta_2 = parser.deserialize_mxl_meta_data()?;
    assert_eq!(borrowed_score_2.notes.len(), 106);
    assert_eq!(borrowed_meta_2.title.as_str(), "Für Elise");

    Ok(())
}
