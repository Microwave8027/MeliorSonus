use crate::notes::*;
use crate::prelude::*;
use crate::rms_dbfs::*;

#[derive(Clone)]
pub struct NoteFeatureExtractorImpl;

impl DspCallBack for NoteFeatureExtractorImpl {
    fn dsp_callback(
        &mut self,
        CallBackParameters {
            buffer,
            cfg,
            filter,
            instrument,
            mpm,
        }: CallBackParameters,
    ) -> () {
        let filtered_frame = filter.process_frames(*buffer);
        let tau = mpm.mpm(&filtered_frame, cfg, instrument);
        let dbfs = loudness(*buffer);
        let note = Note::get_note(cfg.sample_rate, tau, dbfs);
    }
}
