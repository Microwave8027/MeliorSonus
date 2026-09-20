/*
 * MELIORSONUS DSP AUDIO PIPELINE & FEATURE EXTRACTION COORDINATOR
 */

use crate::audio_processing::Notes;
use crate::audio_processing::PitchDetectorMode;
use crate::audio_processing::cpal::engine::AudioEngine;
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::processing::functions::filters::band_pass_filter::BandPassFilter;
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::constants::*;
use crate::utils::error_callback::ErrorCallback;
use crate::utils::global_settings::GlobalSettings;
use cpal::StreamConfig;
use rtrb::Producer;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};

pub static ONNX_MODEL_PATH: OnceLock<String> = OnceLock::new();
pub static TFLITE_MODEL_PATH: OnceLock<String> = OnceLock::new();

pub static BYTEDANCE_ONNX_MODEL_PATH: OnceLock<String> = OnceLock::new();
pub static BYTEDANCE_TFLITE_MODEL_PATH: OnceLock<String> = OnceLock::new();

/// Sets the global static model paths for Spotify Basic Pitch.
pub fn set_basic_pitch_model_paths(tflite_runtime_model: String, onnx_runtime_model: String) {
    let _ = TFLITE_MODEL_PATH.set(tflite_runtime_model);
    let _ = ONNX_MODEL_PATH.set(onnx_runtime_model);
}

/// Sets the global static model paths for ByteDance CRNN ("Byte Dancer").
pub fn set_bytedance_model_paths(tflite_runtime_model: String, onnx_runtime_model: String) {
    let _ = BYTEDANCE_TFLITE_MODEL_PATH.set(tflite_runtime_model);
    let _ = BYTEDANCE_ONNX_MODEL_PATH.set(onnx_runtime_model);
}

pub struct CallBackParameters<'a> {
    pub buffer: &'a [f32; FRAME_SIZE],
    pub cfg: &'a StreamConfig,
    pub filter: &'a mut BandPassFilter,
    pub instrument: &'a Instrument,
    pub mpm: &'a mut MPM,
}

pub trait DspCallBack: Send + 'static + Sized {
    fn new(
        note_rb: Producer<Notes>,
        sample_rate: u32,
        silence_threshold_dbfs: f32,
        pitch_detector_mode: PitchDetectorMode,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Result<Self, Box<dyn Error>>;
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

pub struct Dsp {
    pub instrument: Instrument,
    pub silence_threshold: f32,
    error_callback: Arc<dyn ErrorCallback>,
    pub audio_engine: AudioEngine,
    pub global_settings: Arc<Mutex<GlobalSettings>>,
}

impl Dsp {
    pub fn new(
        instrument: Instrument,
        silence_threshold: f32,
        error_callback: Arc<dyn ErrorCallback>,
        pitch_detector_mode: PitchDetectorMode,
        tflite_runtime_model: String,
        onnx_runtime_model: String,
        note_rb_prod: Producer<Notes>,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Self {
        let _ = TFLITE_MODEL_PATH.set(tflite_runtime_model);
        let _ = ONNX_MODEL_PATH.set(onnx_runtime_model);
        let audio_engine = AudioEngine::new(
            instrument,
            Arc::clone(&error_callback),
            note_rb_prod,
            silence_threshold,
            pitch_detector_mode,
            Arc::clone(&global_settings),
        );

        Self {
            instrument,
            silence_threshold,
            error_callback,
            audio_engine,
            global_settings,
        }
    }

    pub fn new_with_models(
        instrument: Instrument,
        silence_threshold: f32,
        error_callback: Arc<dyn ErrorCallback>,
        pitch_detector_mode: PitchDetectorMode,
        basic_pitch_tflite: String,
        basic_pitch_onnx: String,
        bytedance_tflite: String,
        bytedance_onnx: String,
        note_rb_prod: Producer<Notes>,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Self {
        let _ = TFLITE_MODEL_PATH.set(basic_pitch_tflite);
        let _ = ONNX_MODEL_PATH.set(basic_pitch_onnx);
        let _ = BYTEDANCE_TFLITE_MODEL_PATH.set(bytedance_tflite);
        let _ = BYTEDANCE_ONNX_MODEL_PATH.set(bytedance_onnx);
        let audio_engine = AudioEngine::new(
            instrument,
            Arc::clone(&error_callback),
            note_rb_prod,
            silence_threshold,
            pitch_detector_mode,
            Arc::clone(&global_settings),
        );

        Self {
            instrument,
            silence_threshold,
            error_callback,
            audio_engine,
            global_settings,
        }
    }

    pub fn with_bytedance_models(
        self,
        bytedance_tflite: String,
        bytedance_onnx: String,
    ) -> Self {
        let _ = BYTEDANCE_TFLITE_MODEL_PATH.set(bytedance_tflite);
        let _ = BYTEDANCE_ONNX_MODEL_PATH.set(bytedance_onnx);
        self
    }

    pub fn error_callback(&self) -> &Arc<dyn ErrorCallback> {
        &self.error_callback
    }

    pub fn set_audio_device(&mut self, device_name: Option<String>) {
        self.audio_engine.set_audio_device(device_name);
    }

    pub fn with_audio_device(mut self, device_name: Option<String>) -> Self {
        self.audio_engine.set_audio_device(device_name);
        self
    }

    pub fn start<T: DspCallBack>(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_engine.play::<T>()?;
        Ok(())
    }

    pub fn reset(&mut self, prod: Producer<Notes>) {
        self.audio_engine.reset(prod);
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
}
