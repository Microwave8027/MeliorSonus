use crate::audio_analysis::mxl_metadata::ArchivedMxlMetaData;
use crate::audio_analysis::score_parser::ArchivedSequentialMusicScore;
use memmap2::Mmap;
use std::path::Path;

pub type StaticArcPath = &'static Path;

pub struct BorrowedMxlScore {
    pub(crate) ptr: *const ArchivedSequentialMusicScore,
    pub(crate) _mmap: Mmap,
}

unsafe impl Sync for BorrowedMxlScore {}
unsafe impl Send for BorrowedMxlScore {}

impl std::ops::Deref for BorrowedMxlScore {
    type Target = ArchivedSequentialMusicScore;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

pub struct BorrowedMxlMetaData {
    pub(crate) ptr: *const ArchivedMxlMetaData,
    pub(crate) _mmap: Mmap,
}

unsafe impl Sync for BorrowedMxlMetaData {}
unsafe impl Send for BorrowedMxlMetaData {}

impl std::ops::Deref for BorrowedMxlMetaData {
    type Target = ArchivedMxlMetaData;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}
