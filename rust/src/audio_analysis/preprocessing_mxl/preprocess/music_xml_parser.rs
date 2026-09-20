use super::borrowed::{BorrowedMxlMetaData, BorrowedMxlScore};
use crate::audio_analysis::mxl_metadata::{ArchivedMxlMetaData, MxlMetaData, parse_mxl_metadata};
use crate::audio_analysis::score_parser::{
    ArchivedSequentialMusicScore, SequentialMusicScore, iterate_over_measures,
    iterate_over_measures_for_instrument,
};
use memmap2::MmapOptions;
use musicxml::read_score_timewise;
use rkyv::{access_unchecked, rancor, to_bytes};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

/// mxl path is the actual mxl, target path contains the sequtnial data, and metadata is metadata
pub struct MusicXmlParser {
    mxl_path: Arc<Path>,
    target_mxl_path: Arc<Path>,
    metadata_path: Arc<Path>,
    target_instrument: Option<String>,
}

impl MusicXmlParser {
    pub fn new(
        mxl_path: impl AsRef<Path>,
        target_mxl_path: impl AsRef<Path>,
        metadata_path: impl AsRef<Path>,
    ) -> Self {
        Self {
            mxl_path: Arc::from(mxl_path.as_ref()),
            target_mxl_path: Arc::from(target_mxl_path.as_ref()),
            metadata_path: Arc::from(metadata_path.as_ref()),
            target_instrument: Some("piano".to_string()),
        }
    }

    pub fn new_with_instrument(
        mxl_path: impl AsRef<Path>,
        target_mxl_path: impl AsRef<Path>,
        metadata_path: impl AsRef<Path>,
        target_instrument: impl Into<String>,
    ) -> Self {
        Self {
            mxl_path: Arc::from(mxl_path.as_ref()),
            target_mxl_path: Arc::from(target_mxl_path.as_ref()),
            metadata_path: Arc::from(metadata_path.as_ref()),
            target_instrument: Some(target_instrument.into()),
        }
    }

    pub fn with_target_instrument(mut self, instrument: impl Into<String>) -> Self {
        self.target_instrument = Some(instrument.into());
        self
    }

    pub fn without_instrument_filter(mut self) -> Self {
        self.target_instrument = None;
        self
    }

    pub fn target_instrument(&self) -> Option<&str> {
        self.target_instrument.as_deref()
    }

    pub fn parse_mxl(&self) -> Result<(MxlMetaData, SequentialMusicScore), Box<dyn Error>> {
        // Validate paths at the very beginning
        let mxl_str = self.mxl_path.to_str().ok_or_else(|| {
            format!(
                "Invalid MXL path (not valid UTF-8): {}",
                self.mxl_path.display()
            )
        })?;

        self.validate_paths()?;

        if let Some(parent) = self.metadata_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!(
                    "Metadata parent directory does not exist: {}",
                    parent.display()
                )
                .into());
            }
        }

        let parsed_mxl_content = read_score_timewise(mxl_str)?.content;

        let meta = parse_mxl_metadata(&parsed_mxl_content);

        let sequential_music_score = if let Some(target) = &self.target_instrument {
            iterate_over_measures_for_instrument(
                &parsed_mxl_content.measure,
                &meta.part_list_type,
                &meta.instruments,
                target,
            )
        } else {
            iterate_over_measures(&parsed_mxl_content.measure, &meta.part_list_type)
        };

        Ok((meta, sequential_music_score))
    }

    pub fn parse_mxl_for_instrument(
        &self,
        target_instrument: &str,
    ) -> Result<(MxlMetaData, SequentialMusicScore), Box<dyn Error>> {
        let mxl_str = self.mxl_path.to_str().ok_or_else(|| {
            format!(
                "Invalid MXL path (not valid UTF-8): {}",
                self.mxl_path.display()
            )
        })?;

        self.validate_paths()?;

        if let Some(parent) = self.metadata_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!(
                    "Metadata parent directory does not exist: {}",
                    parent.display()
                )
                .into());
            }
        }

        let parsed_mxl_content = read_score_timewise(mxl_str)?.content;

        let meta = parse_mxl_metadata(&parsed_mxl_content);

        let metadata = meta.clone();

        let sequential_music_score = iterate_over_measures_for_instrument(
            &parsed_mxl_content.measure,
            &meta.part_list_type,
            &meta.instruments,
            target_instrument,
        );

        Ok((metadata, sequential_music_score))
    }

    pub fn validate_paths(&self) -> Result<(), Box<dyn Error>> {
        if !self.mxl_path.exists() {
            return Err(format!("MXL file does not exist: {}", self.mxl_path.display()).into());
        }

        if !self.mxl_path.is_file() {
            return Err(format!("MXL path is not a file: {}", self.mxl_path.display()).into());
        }

        if self.target_mxl_path.as_os_str().is_empty() {
            return Err("Target MXL path cannot be empty".into());
        }

        if let Some(parent) = self.target_mxl_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                return Err(format!(
                    "Target MXL parent directory does not exist: {}",
                    parent.display()
                )
                .into());
            }
        }

        if self.metadata_path.as_os_str().is_empty() {
            return Err("Metadata path cannot be empty".into());
        }

        Ok(())
    }

    pub fn store_mxl(
        &self,
        metadata: MxlMetaData,
        mxl_data: SequentialMusicScore,
    ) -> Result<(), Box<dyn Error>> {
        let mxl_bytes = to_bytes::<rancor::Error>(&mxl_data)?;
        let meta_bytes = to_bytes::<rancor::Error>(&metadata)?;

        let mut mxl_file = File::create(&self.target_mxl_path)?;
        let mut meta_file = File::create(&self.metadata_path)?;

        mxl_file.write_all(&mxl_bytes)?;
        meta_file.write_all(&meta_bytes)?;
        Ok(())
    }

    pub fn parse_and_store_mxl(&self) -> Result<(), Box<dyn Error>> {
        let (meta, mxl) = self.parse_mxl()?;
        self.store_mxl(meta, mxl)?;
        Ok(())
    }

    pub fn deserialize_music_score(&self) -> Result<BorrowedMxlScore, Box<dyn Error>> {
        let file = File::open(&self.target_mxl_path)?;

        // Safety: Unsafe only if another program is writing the file
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let music_score = unsafe { access_unchecked::<ArchivedSequentialMusicScore>(&mmap) };
        Ok(BorrowedMxlScore {
            ptr: music_score as *const ArchivedSequentialMusicScore,
            _mmap: mmap,
        })
    }

    pub fn deserialize_mxl_meta_data(&self) -> Result<BorrowedMxlMetaData, Box<dyn Error>> {
        let file = File::open(&self.metadata_path)?;

        // Safety: Unsafe only if another program is writing the file
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        let music_score = unsafe { access_unchecked::<ArchivedMxlMetaData>(&mmap) };
        Ok(BorrowedMxlMetaData {
            ptr: music_score as *const ArchivedMxlMetaData,
            _mmap: mmap,
        })
    }
}
