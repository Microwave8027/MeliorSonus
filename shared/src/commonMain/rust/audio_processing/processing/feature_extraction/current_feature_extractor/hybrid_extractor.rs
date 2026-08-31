use crate::audio_processing::{HardwareDelegate, PitchDetectionModel};
// note to self: The rubato implementation returns a result error but only does so if the parameters are zero. Make sure not to pass in any zero parameters into the resampler
use crate::audio_processing::dsp::{CallBackParameters, DspCallBack};
use crate::audio_processing::instruments::instrument::{Instrument, InstrumentAcousticProfile};
use crate::audio_processing::instruments::notes::*;
use crate::audio_processing::neural::{
    BasicPitchStub, NeuralTranscriber, SegmentedNoteEvent, StreamingNoteSegmenter,
};
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::audio_processing::processing::functions::pitch::nsdf::NsdfEvaluator;
use crate::audio_processing::processing::functions::resampling::AudioResampler;
use crate::audio_processing::processing::functions::spectral::hfc_onset::HfcOnsetDetector;
use crate::audio_processing::processing::functions::spectral::real_fft::RealFft;
use crate::audio_processing::processing::functions::spectral::spectral_centroid::compute_spectral_centroid;
use crate::audio_processing::processing::functions::time_domain::crest_factor::compute_crest_factor;
use crate::audio_processing::processing::functions::time_domain::keybed_thump::KeybedThumpDetector;
use crate::audio_processing::processing::functions::time_domain::rms_loudness::loudness;
use crate::constants::*;
use crate::prelude::*;
use cpal::StreamConfig;
use rtrb::Producer;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hybrid Real-Time Feature Extractor coordinating:
/// 1. MPM Fast-Path (Native sample rate live pitch tracking & clarity for UI)
/// 2. SIMD DSP (RealFFT centroid & spectral shape, DirectForm2 biquad bandpass filtering)
/// 3. Resampler (Rubato 44.1k/48k -> 22.05k)
/// 4. Neural Transcriber (Spotify Basic Pitch / CRNN via Tract or Stub)
/// 5. Note Segmenter (Discrete StartNote/EndNote extraction with rich MeliorSonus acoustic pedagogy)
/// 6. Lock-Free SPSC Ringbuffer (rtrb::Producer<Notes>)
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
    is_filter_initialized: bool,
    previous_hybrid_mode: Option<HybridPitchDetectorMode>,
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
pub const ONSET_THRESHOLD: f32 = 0.5;
pub const FRAME_THRESHOLD: f32 = 0.4;
pub const PEAK_NSDF_RATIO: f32 = 0.55;

impl HybridFeatureExtractor {
    pub fn set_transcriber(&mut self, transcriber: Box<dyn NeuralTranscriber>) {
        self.transcriber = Some(transcriber);
    }

    pub fn reset_stream_time(&mut self, base: u128) {
        self.base_timestamp = Some(base);
        self.processed_samples = 0;
        self.hfc_onset_detector.reset();
        self.filtered_sliding_window = [0.0; FRAME_SIZE];
        self.is_filter_initialized = false;
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
                self.resampler = Some(AudioResampler::new(
                    self.sample_rate,
                    CRNN_INPUT_SIZE,
                    HOP_SIZE,
                ));
                self.transcriber = Some(Box::new(BasicPitchStub::new(CRNN_INPUT_SIZE)));
            }
            (PitchDetectorMode::Basic, PitchDetectorMode::Crnn)
            | (PitchDetectorMode::Basic, PitchDetectorMode::Hybrid) => {
                self.pitch_detector_mode = mode;
                self.resampler = None;
                self.transcriber = None;
            }
            _ => {}
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

        if let Ok(proceed) = GLOBAL_SETTINGS.try_read() {
            if proceed.show_metrics {
                GLOBAL_AUDIO_METRICS
                    .mpm_freq_bits
                    .store(ph.to_bits(), Ordering::Relaxed);
                GLOBAL_AUDIO_METRICS
                    .clarity_bits
                    .store(pc.to_bits(), Ordering::Relaxed);
            }
        }

        // Note Segmentation & Pedagogical Feature Binding for MPM
        if let Ok(proceed) = GLOBAL_SETTINGS.try_read() {
            if proceed.note_recognition_mode == HybridPitchDetectorMode::Mpm {
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
        // Rubato Resampling (Native -> 22.05 kHz)
        if let Ok(resampled_chunk) = self
            .resampler
            .as_mut()
            .expect("Resampler not initialized")
            .process_chunk(&filtered_frame[..HOP_SIZE])
        {
            // Neural Transcriber (Basic Pitch / Tract CRNN)
            let model_output = self
                .transcriber
                .as_mut()
                .expect("Transcriber not initialized")
                .transcribe_hop(&resampled_chunk, dbfs);

            // Note Segmentation & Pedagogical Feature Binding
            let events = self.segmenter.process_crnn_frame(
                &model_output,
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

impl DspCallBack for HybridFeatureExtractor {
    fn new(
        note_rb: Producer<Notes>,
        sample_rate: u32,
        silence_threshold_dbfs: f32,
        pitch_detector_mode: PitchDetectorMode,
        device: HardwareDelegate,
    ) -> Result<Self, Box<dyn Error>> {
        let mut resampler: Option<AudioResampler> = None;
        let mut transcriber: Option<Box<dyn NeuralTranscriber>> = None;
        let previous_hybrid_mode: Option<HybridPitchDetectorMode> = None;
        if pitch_detector_mode == PitchDetectorMode::Crnn
            || pitch_detector_mode == PitchDetectorMode::Hybrid
        {
            resampler = Some(AudioResampler::new(sample_rate, CRNN_INPUT_SIZE, HOP_SIZE));
            transcriber = Some(Box::new(PitchDetector::new(
                TFLITE_MODEL_PATH.get().unwrap_or(&"".to_string()),
                ONNX_MODEL_PATH.get().unwrap_or(&"".to_string()),
                device,
            )?));
        }
        let segmenter = StreamingNoteSegmenter::new(ONSET_THRESHOLD, FRAME_THRESHOLD);

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
        if let Ok(proceed) = GLOBAL_SETTINGS.try_read() {
            if proceed.show_metrics {
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
