use crate::audio_processing::PitchDetectionModel;
// note to self: The rubato implementation returns a result error but only does so if the parameters are zero. Make sure not to pass in any zero parameters into the resampler
use crate::audio_processing::dsp::{
    BYTEDANCE_ONNX_MODEL_PATH, BYTEDANCE_TFLITE_MODEL_PATH, CallBackParameters, DspCallBack,
    ONNX_MODEL_PATH, TFLITE_MODEL_PATH,
};
use crate::audio_processing::instruments::instrument::{Instrument, InstrumentAcousticProfile};
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::bytedance_crnn::{ByteDanceCrnnModel, ByteDanceCrnnOutput};
use crate::audio_processing::neural::{
    BasicPitchStub, NeuralTranscriber, SegmentedNoteEvent, StreamingNoteSegmenter,
};
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::audio_processing::processing::functions::pitch::nsdf::NsdfEvaluator;
use crate::audio_processing::processing::functions::resampling::AudioResampler;
use crate::audio_processing::processing::functions::spectral::hfc_onset::HfcOnsetDetector;
use crate::audio_processing::processing::functions::spectral::mel_spectrogram::SlaneyMelFrontend;
use crate::audio_processing::processing::functions::spectral::real_fft::RealFft;
use crate::audio_processing::processing::functions::spectral::spectral_centroid::compute_spectral_centroid;
use crate::audio_processing::processing::functions::time_domain::crest_factor::compute_crest_factor;
use crate::audio_processing::processing::functions::time_domain::keybed_thump::KeybedThumpDetector;
use crate::audio_processing::processing::functions::time_domain::rms_loudness::loudness;
use crate::constants::*;
use crate::utils::global_settings::GlobalSettings;
use cpal::StreamConfig;
use rtrb::Producer;
use std::error::Error;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hybrid Real-Time Feature Extractor coordinating:
/// 1. MPM Fast-Path (Native sample rate live pitch tracking & clarity for UI)
/// 2. SIMD DSP (RealFFT centroid & spectral shape, DirectForm2 biquad bandpass filtering)
/// 3. Resampler (Rubato 44.1k/48k -> 22.05k for Basic Pitch, 16k for ByteDance CRNN)
/// 4. Pure-Rust SlaneyMelFrontend (2048-point STFT + 229 Mel filterbanks)
/// 5. Neural Transcriber (ByteDance CRNN 7-Head or Spotify Basic Pitch via LiteRT/Tract)
/// 6. Note Segmenter (Discrete StartNote/EndNote extraction with rich MeliorSonus acoustic pedagogy)
/// 7. Lock-Free SPSC Ringbuffer (rtrb::Producer<Notes>)
pub struct HybridFeatureExtractor {
    pub sample_rate: u32,
    pub note_rb: Producer<Notes>,
    pub real_fft: RealFft,
    pub thump_detector: KeybedThumpDetector,
    pub resampler: Option<AudioResampler>,
    pub transcriber: Option<Box<dyn NeuralTranscriber>>,
    pub segmenter: StreamingNoteSegmenter,
    pub silence_threshold_dbfs: f32,
    pub processed_samples: u64,
    pub base_timestamp: Option<u128>,
    pub pitch_detector_mode: PitchDetectorMode,
    pub hfc_onset_detector: HfcOnsetDetector,
    pub nsdf_evaluator: NsdfEvaluator,
    pub filtered_sliding_window: [f32; FRAME_SIZE],
    pub global_settings: Arc<Mutex<GlobalSettings>>,
    is_filter_initialized: bool,
    previous_hybrid_mode: Option<HybridPitchDetectorMode>,
    // ByteDance CRNN Integration
    pub crnn_type: CrnnType,
    pub bytedance_crnn: Option<ByteDanceCrnnModel>,
    pub resampler_16k: Option<AudioResampler>,
    pub slaney_mel: SlaneyMelFrontend,
    pub pcm_16k_fifo: [f32; 4096],
    pub pcm_16k_len: usize,
    pub mel_context_tensor: [f32; BYTEDANCE_CRNN_CONTEXT_LEN],
}

unsafe impl Send for HybridFeatureExtractor {}

type PitchDetector = PitchDetectionModel;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum PitchDetectorMode {
    Basic,
    Hybrid,
    Crnn,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum HybridPitchDetectorMode {
    Mpm,
    Crnn,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum CrnnType {
    ByteDance,
    BasicPitch,
}

pub const ONSET_THRESHOLD: f32 = 0.5;
pub const FRAME_THRESHOLD: f32 = 0.4;
pub const PEAK_NSDF_RATIO: f32 = 0.55;

impl HybridFeatureExtractor {
    #[inline]
    pub fn set_transcriber(&mut self, transcriber: Box<dyn NeuralTranscriber>) {
        self.transcriber = Some(transcriber);
    }

    #[inline]
    pub fn set_bytedance_crnn(&mut self, crnn: ByteDanceCrnnModel) {
        self.bytedance_crnn = Some(crnn);
    }

    pub fn reset_stream_time(&mut self, base: u128) {
        self.base_timestamp = Some(base);
        self.processed_samples = 0;
        self.hfc_onset_detector.reset();
        self.filtered_sliding_window = [0.0; FRAME_SIZE];
        self.is_filter_initialized = false;
        self.pcm_16k_len = 0;
        self.pcm_16k_fifo.fill(0.0);
        self.mel_context_tensor.fill(-11.5129);
    }

    pub fn compute_frame_timestamp(&mut self, sample_rate: u32) -> u128 {
        let base = match self.base_timestamp {
            Some(ts) => ts,
            None => {
                let ts = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();
                self.base_timestamp = Some(ts);
                ts
            }
        };

        let rate = (sample_rate as u64).max(1);
        let elapsed_ms = (self.processed_samples * 1000) / rate;
        let ts = base + elapsed_ms as u128;
        self.processed_samples += HOP_SIZE as u64;
        ts
    }

    pub fn change_mode(&mut self, mode: PitchDetectorMode) {
        let current_mode = &self.pitch_detector_mode;
        match (&mode, current_mode) {
            (PitchDetectorMode::Crnn, PitchDetectorMode::Basic)
            | (PitchDetectorMode::Hybrid, PitchDetectorMode::Basic) => {
                self.pitch_detector_mode = mode;
                let empty_path = String::new();
                let device = self
                    .global_settings
                    .lock()
                    .map(|s| s.device)
                    .unwrap_or_default();

                match self.crnn_type {
                    CrnnType::ByteDance => {
                        let tflite_path = BYTEDANCE_TFLITE_MODEL_PATH
                            .get()
                            .or_else(|| TFLITE_MODEL_PATH.get())
                            .unwrap_or(&empty_path);
                        let onnx_path = BYTEDANCE_ONNX_MODEL_PATH
                            .get()
                            .or_else(|| ONNX_MODEL_PATH.get())
                            .unwrap_or(&empty_path);

                        self.resampler_16k = Some(AudioResampler::new(
                            self.sample_rate,
                            BYTEDANCE_CRNN_SAMPLE_RATE,
                            HOP_SIZE,
                        ));
                        self.bytedance_crnn =
                            match ByteDanceCrnnModel::new(tflite_path, onnx_path, device) {
                                Ok(model) => Some(model),
                                Err(_) => Some(ByteDanceCrnnModel::new_stub()),
                            };
                    }
                    CrnnType::BasicPitch => {
                        let tflite_path = TFLITE_MODEL_PATH.get().unwrap_or(&empty_path);
                        let onnx_path = ONNX_MODEL_PATH.get().unwrap_or(&empty_path);

                        self.resampler = Some(AudioResampler::new(
                            self.sample_rate,
                            CRNN_INPUT_SIZE,
                            HOP_SIZE,
                        ));
                        self.transcriber = match PitchDetector::new(tflite_path, onnx_path, device)
                        {
                            Ok(model) => Some(Box::new(model)),
                            Err(_) => Some(Box::new(BasicPitchStub::new(CRNN_INPUT_SIZE))),
                        };
                    }
                }
            }
            (PitchDetectorMode::Basic, PitchDetectorMode::Crnn)
            | (PitchDetectorMode::Basic, PitchDetectorMode::Hybrid) => {
                self.pitch_detector_mode = mode;
                self.resampler = None;
                self.transcriber = None;
                self.resampler_16k = None;
                self.bytedance_crnn = None;
            }
            _ => {
                self.pitch_detector_mode = mode;
            }
        }
    }

    pub fn change_crnn_type(&mut self, crnn_type: CrnnType) {
        if self.crnn_type == crnn_type {
            return;
        }
        self.crnn_type = crnn_type;
        if self.pitch_detector_mode == PitchDetectorMode::Basic {
            return;
        }
        let empty_path = String::new();
        let device = self
            .global_settings
            .lock()
            .map(|s| s.device)
            .unwrap_or_default();

        match crnn_type {
            CrnnType::ByteDance => {
                let tflite_path = BYTEDANCE_TFLITE_MODEL_PATH
                    .get()
                    .or_else(|| TFLITE_MODEL_PATH.get())
                    .unwrap_or(&empty_path);
                let onnx_path = BYTEDANCE_ONNX_MODEL_PATH
                    .get()
                    .or_else(|| ONNX_MODEL_PATH.get())
                    .unwrap_or(&empty_path);

                self.resampler = None;
                self.transcriber = None;
                self.resampler_16k = Some(AudioResampler::new(
                    self.sample_rate,
                    BYTEDANCE_CRNN_SAMPLE_RATE,
                    HOP_SIZE,
                ));
                self.bytedance_crnn = match ByteDanceCrnnModel::new(tflite_path, onnx_path, device)
                {
                    Ok(model) => Some(model),
                    Err(_) => Some(ByteDanceCrnnModel::new_stub()),
                };
            }
            CrnnType::BasicPitch => {
                let tflite_path = TFLITE_MODEL_PATH.get().unwrap_or(&empty_path);
                let onnx_path = ONNX_MODEL_PATH.get().unwrap_or(&empty_path);

                self.resampler_16k = None;
                self.bytedance_crnn = None;
                self.resampler = Some(AudioResampler::new(
                    self.sample_rate,
                    CRNN_INPUT_SIZE,
                    HOP_SIZE,
                ));
                self.transcriber = match PitchDetector::new(tflite_path, onnx_path, device) {
                    Ok(model) => Some(Box::new(model)),
                    Err(_) => Some(Box::new(BasicPitchStub::new(CRNN_INPUT_SIZE))),
                };
            }
        }
    }

    pub fn process_basic_path(
        &mut self,
        filtered_frame: &[f32; FRAME_SIZE],
        cfg: &StreamConfig,
        instrument: &Instrument,
        mpm: &mut MPM,
        dbfs: f32,
        hfc_onset_ts: Option<u128>,
        timestamp: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
    ) {
        // Native MPM Fast-Path: Pitch & Clarity Tracking
        let (ph, pc) = mpm.mpm(filtered_frame, cfg, instrument);

        if let Ok(process) = self.global_settings.lock() {
            if process.show_metrics {
                GLOBAL_AUDIO_METRICS
                    .mpm_freq_bits
                    .store(ph.to_bits(), Ordering::Relaxed);
                GLOBAL_AUDIO_METRICS
                    .clarity_bits
                    .store(pc.to_bits(), Ordering::Relaxed);
            }
        }

        // Note Segmentation & Pedagogical Feature Binding for MPM
        if let Ok(process) = self.global_settings.lock() {
            if process.note_recognition_mode == HybridPitchDetectorMode::Mpm {
                let events = self.segmenter.process_mpm_frame(
                    (ph, pc, dbfs),
                    hfc_onset_ts,
                    timestamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    profile,
                );
                for event in events {
                    match event {
                        SegmentedNoteEvent::Start(s) => {
                            let _ = self.note_rb.push(Notes::Start(s));
                        }
                        SegmentedNoteEvent::End(e) => {
                            let _ = self.note_rb.push(Notes::End(e));
                        }
                    }
                }
            }
        }
    }

    /// Converts a 2048-sample 16 kHz audio window into 229 Slaney Log-Mel frequency bins.
    #[inline]
    pub fn convert_audio_to_mel_frame(
        &mut self,
        window_2048: &[f32; BYTEDANCE_CRNN_FFT_SIZE],
        out_mel: &mut [f32; BYTEDANCE_CRNN_MEL_BINS],
    ) {
        self.slaney_mel.compute_log_mel_frame(window_2048, out_mel);
    }

    /// Converts a 2048-sample 16 kHz audio window into 229 Log-Mel bins,
    /// shifts the rolling 32-frame context tensor, and runs neural inference
    /// to output all 7 ByteDance CRNN tensors.
    #[inline]
    pub fn convert_audio_to_crnn_output(
        &mut self,
        window_2048: &[f32; BYTEDANCE_CRNN_FFT_SIZE],
        dbfs: f32,
    ) -> Option<ByteDanceCrnnOutput> {
        if self.bytedance_crnn.is_none() {
            return None;
        }

        let mut new_mel_frame = [0.0f32; BYTEDANCE_CRNN_MEL_BINS];
        self.convert_audio_to_mel_frame(window_2048, &mut new_mel_frame);

        // Shift rolling context buffer by 1 frame (229 floats)
        self.mel_context_tensor
            .copy_within(BYTEDANCE_CRNN_MEL_BINS..BYTEDANCE_CRNN_CONTEXT_LEN, 0);
        self.mel_context_tensor[BYTEDANCE_CRNN_CONTEXT_LEN - BYTEDANCE_CRNN_MEL_BINS..]
            .copy_from_slice(&new_mel_frame);

        // Run 7-head acoustic neural inference
        self.bytedance_crnn
            .as_mut()
            .map(|crnn| crnn.transcribe_spectrogram_frame(&self.mel_context_tensor, dbfs))
    }

    /// Processes a single ByteDance CRNN frame: converts the 2048-sample audio window
    /// into CRNN output via `convert_audio_to_crnn_output`, passes the 7-head predictions
    /// into `StreamingNoteSegmenter::process_bytedance_crnn_frame`, and pushes segmented note
    /// events to the ring buffer.
    pub fn process_bytedance_crnn_frame(
        &mut self,
        window_2048: &[f32; BYTEDANCE_CRNN_FFT_SIZE],
        dbfs: f32,
        timestamp: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
        filtered_frame: &[f32; FRAME_SIZE],
    ) {
        if let Some(crnn_out) = self.convert_audio_to_crnn_output(window_2048, dbfs) {
            let events = self.segmenter.process_bytedance_crnn_frame(
                &crnn_out,
                timestamp,
                crest_factor,
                sub_thump_dbfs,
                spectral_centroid,
                profile,
                filtered_frame,
            );

            for event in events {
                match event {
                    SegmentedNoteEvent::Start(s) => {
                        let _ = self.note_rb.push(Notes::Start(s));
                    }
                    SegmentedNoteEvent::End(e) => {
                        let _ = self.note_rb.push(Notes::End(e));
                    }
                }
            }
        }
    }

    pub fn process_crnn_path(
        &mut self,
        filtered_frame: &[f32; FRAME_SIZE],
        dbfs: f32,
        timestamp: u128,
        crest_factor: f32,
        sub_thump_dbfs: f32,
        spectral_centroid: f32,
        profile: &InstrumentAcousticProfile,
    ) {
        match self.crnn_type {
            CrnnType::ByteDance => {
                if let Some(ref mut resampler) = self.resampler_16k {
                    if let Ok(resampled_16k) = resampler.process_chunk(&filtered_frame[..HOP_SIZE])
                    {
                        if self.pcm_16k_len + resampled_16k.len() > 4096 {
                            let excess = (self.pcm_16k_len + resampled_16k.len()) - 4096;
                            self.pcm_16k_fifo.copy_within(excess..self.pcm_16k_len, 0);
                            self.pcm_16k_len -= excess;
                        }
                        let copy_len = resampled_16k.len().min(4096 - self.pcm_16k_len);
                        self.pcm_16k_fifo[self.pcm_16k_len..self.pcm_16k_len + copy_len]
                            .copy_from_slice(&resampled_16k[..copy_len]);
                        self.pcm_16k_len += copy_len;

                        while self.pcm_16k_len >= BYTEDANCE_CRNN_FFT_SIZE {
                            let mut window_2048 = [0.0f32; BYTEDANCE_CRNN_FFT_SIZE];
                            window_2048
                                .copy_from_slice(&self.pcm_16k_fifo[..BYTEDANCE_CRNN_FFT_SIZE]);

                            self.process_bytedance_crnn_frame(
                                &window_2048,
                                dbfs,
                                timestamp,
                                crest_factor,
                                sub_thump_dbfs,
                                spectral_centroid,
                                profile,
                                filtered_frame,
                            );

                            // Advance FIFO by 320 samples
                            self.pcm_16k_fifo
                                .copy_within(BYTEDANCE_CRNN_HOP_SIZE..self.pcm_16k_len, 0);
                            self.pcm_16k_len -= BYTEDANCE_CRNN_HOP_SIZE;
                        }
                    }
                }
            }
            CrnnType::BasicPitch => {
                if let (Some(resampler), Some(transcriber)) =
                    (&mut self.resampler, &mut self.transcriber)
                {
                    if let Ok(resampled_chunk) =
                        resampler.process_chunk(&filtered_frame[..HOP_SIZE])
                    {
                        let model_output = transcriber.transcribe_hop(&resampled_chunk, dbfs);
                        let events = self.segmenter.process_crnn_frame(
                            &model_output,
                            timestamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            profile,
                            filtered_frame,
                        );

                        for event in events {
                            match event {
                                SegmentedNoteEvent::Start(s) => {
                                    let _ = self.note_rb.push(Notes::Start(s));
                                }
                                SegmentedNoteEvent::End(e) => {
                                    let _ = self.note_rb.push(Notes::End(e));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl DspCallBack for HybridFeatureExtractor {
    fn new(
        note_rb: Producer<Notes>,
        sample_rate: u32,
        silence_threshold_dbfs: f32,
        pitch_detector_mode: PitchDetectorMode,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut resampler: Option<AudioResampler> = None;
        let mut transcriber: Option<Box<dyn NeuralTranscriber>> = None;
        let mut resampler_16k: Option<AudioResampler> = None;
        let mut bytedance_crnn: Option<ByteDanceCrnnModel> = None;
        let mut crnn_type = CrnnType::BasicPitch;
        let previous_hybrid_mode: Option<HybridPitchDetectorMode> = None;

        if pitch_detector_mode == PitchDetectorMode::Crnn
            || pitch_detector_mode == PitchDetectorMode::Hybrid
        {
            let Ok(setting) = global_settings.lock() else {
                return Err(
                    "Global Settings is poisoned or being stalled by another thread".into(),
                );
            };
            crnn_type = setting.crnn_type;
            let empty_path = String::new();

            match crnn_type {
                CrnnType::ByteDance => {
                    let tflite_path = BYTEDANCE_TFLITE_MODEL_PATH
                        .get()
                        .or_else(|| TFLITE_MODEL_PATH.get())
                        .unwrap_or(&empty_path);
                    let onnx_path = BYTEDANCE_ONNX_MODEL_PATH
                        .get()
                        .or_else(|| ONNX_MODEL_PATH.get())
                        .unwrap_or(&empty_path);

                    resampler_16k = Some(AudioResampler::new(
                        sample_rate,
                        BYTEDANCE_CRNN_SAMPLE_RATE,
                        HOP_SIZE,
                    ));
                    bytedance_crnn =
                        match ByteDanceCrnnModel::new(tflite_path, onnx_path, setting.device) {
                            Ok(model) => Some(model),
                            Err(_) => Some(ByteDanceCrnnModel::new_stub()),
                        };
                }
                CrnnType::BasicPitch => {
                    let tflite_path = TFLITE_MODEL_PATH.get().unwrap_or(&empty_path);
                    let onnx_path = ONNX_MODEL_PATH.get().unwrap_or(&empty_path);

                    resampler = Some(AudioResampler::new(sample_rate, CRNN_INPUT_SIZE, HOP_SIZE));
                    transcriber = Some(Box::new(PitchDetector::new(
                        tflite_path,
                        onnx_path,
                        setting.device,
                    )?));
                }
            }
        }
        let segmenter = StreamingNoteSegmenter::new(ONSET_THRESHOLD, FRAME_THRESHOLD, sample_rate);

        Ok(Self {
            sample_rate,
            note_rb,
            real_fft: RealFft::new(),
            thump_detector: KeybedThumpDetector::new(sample_rate),
            resampler,
            transcriber,
            segmenter,
            silence_threshold_dbfs,
            processed_samples: 0,
            base_timestamp: None,
            pitch_detector_mode,
            hfc_onset_detector: HfcOnsetDetector::default_detector(),
            nsdf_evaluator: NsdfEvaluator::new(),
            filtered_sliding_window: [0.0; FRAME_SIZE],
            is_filter_initialized: false,
            previous_hybrid_mode,
            global_settings,
            crnn_type,
            bytedance_crnn,
            resampler_16k,
            slaney_mel: SlaneyMelFrontend::new(),
            pcm_16k_fifo: [0.0; 4096],
            pcm_16k_len: 0,
            mel_context_tensor: [-11.5129; BYTEDANCE_CRNN_CONTEXT_LEN],
        })
    }

    fn dsp_callback(
        &mut self,
        CallBackParameters {
            buffer,
            cfg,
            filter,
            instrument,
            mpm,
        }: CallBackParameters,
    ) {
        let timestamp = self.compute_frame_timestamp(cfg.sample_rate);
        let profile = instrument.acoustic_profile();

        let new_raw_hop: &[f32; HOP_SIZE] = buffer[FRAME_SIZE - HOP_SIZE..]
            .try_into()
            .expect("Buffer slice matches HOP_SIZE");

        // Continuous IIR Band-Pass Filtering via Sliding Window FIFO
        if !self.is_filter_initialized {
            self.filtered_sliding_window = filter.process_frames(buffer);
            self.is_filter_initialized = true;
        } else {
            self.filtered_sliding_window
                .copy_within(HOP_SIZE..FRAME_SIZE, 0);
            let filtered_hop = filter.process_hop(new_raw_hop);
            self.filtered_sliding_window[FRAME_SIZE - HOP_SIZE..].copy_from_slice(&filtered_hop);
        }

        let filtered_frame = self.filtered_sliding_window;
        let dbfs = loudness(filtered_frame.as_slice());
        let crest_factor = compute_crest_factor(&filtered_frame);
        let sub_thump_dbfs = self.thump_detector.evaluate_hop(new_raw_hop);

        // SIMD RealFFT Spectral Shape & Centroid
        self.real_fft.compute_magnitude_spectrum(&filtered_frame);
        let spectral_centroid =
            compute_spectral_centroid(&self.real_fft.magnitude, cfg.sample_rate as f32);
        let hfc_onset_ts = self.hfc_onset_detector.process_frame(
            self.real_fft.magnitude.as_slice(),
            dbfs,
            timestamp,
        );

        // Always update telemetry metrics per incoming frame
        if let Ok(process) = self.global_settings.lock() {
            if process.show_metrics {
                GLOBAL_AUDIO_METRICS
                    .rms_dbfs_bits
                    .store(dbfs.to_bits(), Ordering::Relaxed);
                GLOBAL_AUDIO_METRICS
                    .processed_frames
                    .fetch_add(1, Ordering::Relaxed);
            }
        }

        // Check Silence Gating
        if dbfs < self.silence_threshold_dbfs {
            let finalized = self.segmenter.finalize_all(timestamp, &profile);
            for end_note in finalized {
                let _ = self.note_rb.push(Notes::End(end_note));
            }
            self.pcm_16k_len = 0;
            return;
        }

        match self.pitch_detector_mode {
            PitchDetectorMode::Basic => {
                self.process_basic_path(
                    &filtered_frame,
                    cfg,
                    instrument,
                    mpm,
                    dbfs,
                    hfc_onset_ts,
                    timestamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    &profile,
                );
            }
            PitchDetectorMode::Crnn => {
                self.process_crnn_path(
                    &filtered_frame,
                    dbfs,
                    timestamp,
                    crest_factor,
                    sub_thump_dbfs,
                    spectral_centroid,
                    &profile,
                );
            }
            PitchDetectorMode::Hybrid => {
                let (nsdf_clarity, nsdf_peak_ratio) =
                    self.nsdf_evaluator.evaluate_frame(&filtered_frame);

                let mpm_cfg = instrument.mpm_config();
                let is_monophonic =
                    nsdf_clarity >= mpm_cfg.clarity_threshold && nsdf_peak_ratio <= PEAK_NSDF_RATIO;

                let target_mode = if is_monophonic {
                    HybridPitchDetectorMode::Mpm
                } else {
                    HybridPitchDetectorMode::Crnn
                };

                let previous_mode = self.previous_hybrid_mode;
                self.previous_hybrid_mode = Some(target_mode);

                match (previous_mode, target_mode) {
                    (Some(HybridPitchDetectorMode::Mpm), HybridPitchDetectorMode::Crnn) => {
                        // Switching from monophonic to CRNN: push active note in MPM to CRNN
                        self.segmenter.transfer_mono_to_poly();
                        self.process_crnn_path(
                            &filtered_frame,
                            dbfs,
                            timestamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            &profile,
                        );
                    }
                    (Some(HybridPitchDetectorMode::Crnn), HybridPitchDetectorMode::Mpm) => {
                        // Switching from CRNN to monophonic: push last active note in CRNN that MPM detects into MPM and finalize everything else
                        let (ph, pc) = mpm.mpm(&filtered_frame, cfg, instrument);
                        let detected_note = if pc >= self.segmenter.frame_threshold() {
                            get_note(ph).map(|(p, o, _)| (p, o))
                        } else {
                            None
                        };
                        let finalized = self.segmenter.transfer_poly_to_mono(
                            detected_note,
                            timestamp,
                            &profile,
                        );
                        for end_note in finalized {
                            let _ = self.note_rb.push(Notes::End(end_note));
                        }
                        self.process_basic_path(
                            &filtered_frame,
                            cfg,
                            instrument,
                            mpm,
                            dbfs,
                            hfc_onset_ts,
                            timestamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            &profile,
                        );
                    }
                    (_, HybridPitchDetectorMode::Mpm) => {
                        self.process_basic_path(
                            &filtered_frame,
                            cfg,
                            instrument,
                            mpm,
                            dbfs,
                            hfc_onset_ts,
                            timestamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            &profile,
                        );
                    }
                    (_, HybridPitchDetectorMode::Crnn) => {
                        self.process_crnn_path(
                            &filtered_frame,
                            dbfs,
                            timestamp,
                            crest_factor,
                            sub_thump_dbfs,
                            spectral_centroid,
                            &profile,
                        );
                    }
                }
            }
        }
    }
}
