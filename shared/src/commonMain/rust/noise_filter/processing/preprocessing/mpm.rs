use crate::high_pass_filter::BandPassFilter;
use crate::notes::*;
use crate::prelude::*;

pub struct FeatureExtractorImpl;

impl DspCallBack for FeatureExtractorImpl {
    fn dsp_callback(
        &self,
        buffer: [f32; FRAME_SIZE],
        cfg: &StreamConfig,
        filter: &mut BandPassFilter,
    ) -> () {
        let filtered_frame = filter.process_frames(buffer);
    }
}

impl FeatureExtractorImpl {
    pub fn yin(frame: [f32; FRAME_SIZE], cfg: StreamConfig) -> Note {
        todo!()
    }
}
