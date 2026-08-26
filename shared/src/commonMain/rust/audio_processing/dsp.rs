/*
 * MELIORSONUS DSP AUDIO PIPELINE & FEATURE EXTRACTION COORDINATOR
 */

use crate::audio_processing::cpal::engine::AudioEngine;
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::Notes;
use crate::audio_processing::processing::feature_extraction::dsp_feature_extractor::DspFeatureExtractor;
use crate::audio_processing::processing::feature_extraction::monophonic_feature_extractor::NoteFeatureExtractorImpl;
use crate::audio_processing::processing::feature_extraction::polyphonic_feature_extractor::PolyphonicFeatureExtractorImpl;
use crate::audio_processing::processing::functions::high_pass_filter::BandPassFilter;
use crate::audio_processing::processing::functions::mpm::MPM;
use crate::constants::*;
use crate::utils::error_callback::ErrorCallback;
use cpal::StreamConfig;
use ringbuf::traits::*;
use ringbuf::{HeapCons, HeapRb};
use std::error::Error;
use std::sync::Arc;

pub struct CallBackParameters<'a> {
    pub buffer: &'a [f32; FRAME_SIZE],
    pub cfg: &'a StreamConfig,
    pub filter: &'a mut BandPassFilter,
    pub instrument: &'a Instrument,
    pub mpm: &'a mut MPM,
}

pub trait DspCallBack: Send + 'static {
    fn dsp_callback(
        &mut self,
        CallBackParameters {
            buffer: _,
            cfg: _,
            filter: _,
            instrument: _,
            mpm: _,
        }: CallBackParameters,
    ) {
    }
}

pub struct Dsp<T: ErrorCallback> {
    pub instrument: Instrument,
    pub silence_threshold: f32,
    rb_cons: Option<HeapCons<Notes>>,
    error_callback: Arc<T>,
    pub audio_engine: AudioEngine<DspFeatureExtractor, T>,
}

impl<T: ErrorCallback> Dsp<T> {
    pub fn new(instrument: Instrument, silence_threshold: f32, error_callback: Arc<T>) -> Self {
        let rb = HeapRb::<Notes>::new(NOTE_RINGBUF_CAPACITY);
        let (prod, cons) = rb.split();
        let feature_extractor = DspFeatureExtractor::new(
            prod,
            NoteFeatureExtractorImpl::new(silence_threshold),
            PolyphonicFeatureExtractorImpl::new(),
            silence_threshold,
        );
        let audio_engine =
            AudioEngine::new(instrument, feature_extractor, Arc::clone(&error_callback));

        Self {
            instrument,
            silence_threshold,
            rb_cons: Some(cons),
            error_callback,
            audio_engine,
        }
    }

    pub fn error_callback(&self) -> &Arc<T> {
        &self.error_callback
    }

    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_engine.play()?;
        Ok(())
    }

    pub fn reset(&mut self) {
        let fe = self.create_new_feature_extractor();
        self.audio_engine.reset(fe);
    }

    fn create_new_feature_extractor(&mut self) -> DspFeatureExtractor {
        let rb = HeapRb::new(NOTE_RINGBUF_CAPACITY);
        let (prod, cons) = rb.split();
        self.rb_cons = Some(cons);

        DspFeatureExtractor::new(
            prod,
            NoteFeatureExtractorImpl::new(self.silence_threshold),
            PolyphonicFeatureExtractorImpl::new(),
            self.silence_threshold,
        )
    }

    pub fn pause(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_engine.pause()?;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_engine.resume()?;
        Ok(())
    }

    pub fn stop(mut self) {
        self.audio_engine.end();
    }

    pub fn take_consumer(&mut self) -> Option<HeapCons<Notes>> {
        self.rb_cons.take()
    }

    pub fn pop_note(&mut self) -> Option<Notes> {
        self.rb_cons.as_mut().and_then(|cons| cons.try_pop())
    }
}
