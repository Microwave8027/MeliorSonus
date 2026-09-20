use cpal::{BufferSize, StreamConfig};
use rtrb::Producer;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio_processing::PitchDetectorMode;
use crate::audio_processing::dsp::{CallBackParameters, DspCallBack};
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::Notes;
use crate::audio_processing::processing::functions::filters::band_pass_filter::BandPassFilter;
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::constants::{FRAME_SIZE, HANGOVER_FRAMES_DEFAULT, HOP_SIZE};
use crate::utils::error_callback::ErrorCallback;
use crate::utils::errors::{AudioEngineError, RustError};
use crate::utils::global_settings::GlobalSettings;
use crate::utils::guard::DropGuard;

enum EngineSignal {
    Paused,
    Playing,
    Closed,
}

pub struct WavReader {
    pub file_path: PathBuf,
    pub real_time: bool,
    signal_tx: Option<mpsc::Sender<EngineSignal>>,
    pub is_playing: Arc<AtomicBool>,
    error_callback: Arc<dyn ErrorCallback>,
    supervisor_handle: Option<thread::JoinHandle<()>>,
    instrument: Arc<Instrument>,
    rt_rb_prod: Option<Producer<Notes>>,
    silence_threshold: f32,
    pitch_detector_mode: PitchDetectorMode,
    pub global_settings: Arc<Mutex<GlobalSettings>>,
}

impl WavReader {
    pub fn new<P: AsRef<Path>>(
        file_path: P,
        instrument: Instrument,
        error_callback: Arc<dyn ErrorCallback>,
        rtrb_prod: Producer<Notes>,
        silence_threshold: f32,
        pitch_detector_mode: PitchDetectorMode,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            real_time: true,
            signal_tx: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            error_callback,
            supervisor_handle: None,
            instrument: Arc::new(instrument),
            rt_rb_prod: Some(rtrb_prod),
            silence_threshold,
            pitch_detector_mode,
            global_settings,
        }
    }

    pub fn set_file_path<P: AsRef<Path>>(&mut self, path: P) {
        self.file_path = path.as_ref().to_path_buf();
    }

    pub fn with_file_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.file_path = path.as_ref().to_path_buf();
        self
    }

    pub fn set_real_time(&mut self, real_time: bool) {
        self.real_time = real_time;
    }

    pub fn with_real_time(mut self, real_time: bool) -> Self {
        self.real_time = real_time;
        self
    }

    pub fn error_callback(&self) -> &Arc<dyn ErrorCallback> {
        &self.error_callback
    }

    pub fn play<T: DspCallBack>(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_playing.load(Ordering::Relaxed) {
            return Err("Reset WavReader and read errors before playing again".into());
        }

        if self
            .supervisor_handle
            .as_ref()
            .is_some_and(|h| !h.is_finished())
        {
            return Err("WavReader is already playing".into());
        }

        // Verify a producer was supplied before touching the filesystem
        if self.rt_rb_prod.is_none() {
            return Err(
                "No producer was supplied, please reset WavReader before playing again".into(),
            );
        }

        if !self.file_path.exists() {
            return Err(format!("File does not exist: {}", self.file_path.display()).into());
        }

        // Open and validate WAV format before consuming producer
        let mut reader = ::hound::WavReader::open(&self.file_path)
            .map_err(|e| format!("Failed to open WAV file: {}", e))?;
        let spec = reader.spec();
        if spec.channels == 0 {
            return Err("WAV file has 0 channels".into());
        }
        if spec.sample_rate < 8000 {
            return Err(format!(
                "Unsupported sample rate: {} Hz (minimum 8000 Hz required)",
                spec.sample_rate
            )
            .into());
        }
        if reader.len() == 0 {
            return Err("WAV file contains no audio samples".into());
        }

        match (spec.sample_format, spec.bits_per_sample) {
            (::hound::SampleFormat::Float, 32)
            | (::hound::SampleFormat::Int, 16)
            | (::hound::SampleFormat::Int, 24)
            | (::hound::SampleFormat::Int, 32)
            | (::hound::SampleFormat::Int, 8) => {}
            _ => {
                return Err(format!(
                    "Unsupported WAV format: {:?} with {} bits per sample",
                    spec.sample_format, spec.bits_per_sample
                )
                .into());
            }
        }

        let rb_prod = self.rt_rb_prod.take().unwrap();
        let (tx, rx) = mpsc::channel();
        let instrument = Arc::clone(&self.instrument);
        let error_callback = Arc::clone(&self.error_callback);
        let threshold = self.silence_threshold;
        let mode = self.pitch_detector_mode;
        let global_settings = Arc::clone(&self.global_settings);
        let real_time = self.real_time;

        self.signal_tx = Some(tx);
        self.is_playing.store(true, Ordering::Relaxed);
        let is_playing = Arc::clone(&self.is_playing);

        let handle = thread::spawn(move || {
            crate::utils::enable_flush_to_zero();
            let _guard = DropGuard::from(Arc::clone(&is_playing));

            let sample_rate = spec.sample_rate;
            let samples = match decode_wav_samples(&mut reader) {
                Ok(s) => s,
                Err(e) => {
                    is_playing.store(false, Ordering::SeqCst);
                    error_callback.on_error(RustError::AudioEngineError(
                        AudioEngineError::StreamBuildError(e.to_string()),
                    ));
                    return;
                }
            };

            let config = StreamConfig {
                channels: 1,
                sample_rate,
                buffer_size: BufferSize::Fixed(FRAME_SIZE as u32),
            };

            let mut frame = [0.0f32; FRAME_SIZE];
            let mut irr_filter_inst = BandPassFilter::new(
                instrument.filter_range().hpf_cutoff_hz,
                instrument.filter_range().harmonic_ceiling_hz,
                sample_rate,
            );
            let mut mpm = MPM::new(FRAME_SIZE / 2);

            let mut feature_extractor =
                match T::new(rb_prod, sample_rate, threshold, mode, global_settings) {
                    Ok(v) => v,
                    Err(err) => {
                        is_playing.store(false, Ordering::SeqCst);
                        error_callback
                            .on_error(RustError::FeatureExtactorBuildError(err.to_string()));
                        return;
                    }
                };

            let mut is_paused = false;
            let mut offset = 0;
            let total_span = samples.len() + (HANGOVER_FRAMES_DEFAULT as usize) * HOP_SIZE;
            let hop_duration = Duration::from_secs_f64(HOP_SIZE as f64 / sample_rate as f64);
            let mut next_tick = std::time::Instant::now();

            'supervisor: loop {
                loop {
                    match rx.try_recv() {
                        Ok(msg) => match msg {
                            EngineSignal::Closed => break 'supervisor,
                            EngineSignal::Paused => is_paused = true,
                            EngineSignal::Playing => is_paused = false,
                        },
                        Err(TryRecvError::Disconnected) => break 'supervisor,
                        Err(TryRecvError::Empty) => break,
                    }
                }

                if is_paused {
                    thread::sleep(Duration::from_millis(10));
                    next_tick = std::time::Instant::now();
                    continue 'supervisor;
                }

                if offset >= total_span {
                    is_playing.store(false, Ordering::SeqCst);
                    error_callback.on_complete();
                    break 'supervisor;
                }

                if real_time {
                    let now = std::time::Instant::now();
                    if next_tick > now {
                        thread::sleep(next_tick - now);
                        next_tick += hop_duration;
                    } else {
                        next_tick = now + hop_duration;
                    }
                }

                frame.fill(0.0);
                if offset < samples.len() {
                    let end = (offset + FRAME_SIZE).min(samples.len());
                    frame[..end - offset].copy_from_slice(&samples[offset..end]);
                }

                feature_extractor.dsp_callback(CallBackParameters {
                    buffer: &frame,
                    cfg: &config,
                    filter: &mut irr_filter_inst,
                    instrument: &instrument,
                    mpm: &mut mpm,
                });

                offset += HOP_SIZE;
            }
        });

        self.supervisor_handle = Some(handle);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.is_playing.load(Ordering::Relaxed) {
            return Ok(());
        }
        let Some(handle) = &self.supervisor_handle else {
            return Err("WavReader not initialized or already finished".into());
        };
        if handle.is_finished() {
            return Ok(());
        }
        if let Some(tx) = &self.signal_tx {
            tx.send(EngineSignal::Paused)?;
            self.is_playing.store(false, Ordering::Relaxed);
            Ok(())
        } else {
            Err("WavReader not initialized".into())
        }
    }

    pub fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_playing.load(Ordering::Relaxed) {
            return Ok(());
        }
        let Some(handle) = &self.supervisor_handle else {
            return Err("WavReader not initialized or already finished".into());
        };
        if handle.is_finished() {
            return Err("WavReader has finished playing".into());
        }
        if let Some(tx) = &self.signal_tx {
            tx.send(EngineSignal::Playing)?;
            self.is_playing.store(true, Ordering::Relaxed);
            Ok(())
        } else {
            Err("WavReader not initialized".into())
        }
    }

    pub fn reset(&mut self, rtrb_prod: Producer<Notes>) {
        self.end();
        self.is_playing.store(false, Ordering::Relaxed);
        self.rt_rb_prod = Some(rtrb_prod);
    }

    pub fn end(&mut self) {
        if let Some(tx) = self.signal_tx.take() {
            let _ = tx.send(EngineSignal::Closed);
        }
        if let Some(handle) = self.supervisor_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn wait(&mut self) {
        if let Some(handle) = self.supervisor_handle.take() {
            let _ = handle.join();
        }
        self.signal_tx = None;
    }
}

impl Drop for WavReader {
    fn drop(&mut self) {
        self.end();
    }
}

fn decode_samples_to_mono<S, F>(
    mut samples: impl Iterator<Item = Result<S, ::hound::Error>>,
    channels: usize,
    num_frames: usize,
    scale_fn: F,
) -> Result<Vec<f32>, Box<dyn Error>>
where
    F: Fn(S) -> f32,
{
    let mut mono = Vec::with_capacity(num_frames);

    if channels == 1 {
        for s in samples {
            mono.push(scale_fn(s?));
        }
    } else if channels == 2 {
        while let Some(s0) = samples.next() {
            let s1 = samples.next().ok_or("Unexpected end of WAV data")?;
            let v0 = scale_fn(s0?);
            let v1 = scale_fn(s1?);
            mono.push((v0 + v1) * 0.5);
        }
    } else {
        let inv_ch = 1.0 / channels as f32;
        for _ in 0..num_frames {
            let mut sum = 0.0f32;
            for _ in 0..channels {
                let s = samples.next().ok_or("Unexpected end of WAV data")?;
                sum += scale_fn(s?);
            }
            mono.push(sum * inv_ch);
        }
    }

    Ok(mono)
}

fn decode_wav_samples(
    reader: &mut ::hound::WavReader<BufReader<File>>,
) -> Result<Vec<f32>, Box<dyn Error>> {
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let num_frames = reader.duration() as usize;

    match (spec.sample_format, spec.bits_per_sample) {
        (::hound::SampleFormat::Float, 32) => {
            decode_samples_to_mono(reader.samples::<f32>(), channels, num_frames, |s| s)
        }
        (::hound::SampleFormat::Int, 16) => {
            decode_samples_to_mono(reader.samples::<i16>(), channels, num_frames, |s| {
                s as f32 / 32768.0
            })
        }
        (::hound::SampleFormat::Int, 24) => {
            let scale = (1 << 23) as f32;
            decode_samples_to_mono(reader.samples::<i32>(), channels, num_frames, move |s| {
                s as f32 / scale
            })
        }
        (::hound::SampleFormat::Int, 32) => {
            decode_samples_to_mono(reader.samples::<i32>(), channels, num_frames, |s| {
                s as f32 / 2147483648.0
            })
        }
        (::hound::SampleFormat::Int, 8) => {
            decode_samples_to_mono(reader.samples::<i8>(), channels, num_frames, |s| {
                s as f32 / 128.0
            })
        }
        _ => Err(format!(
            "Unsupported WAV format: {:?} with {} bits per sample",
            spec.sample_format, spec.bits_per_sample
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_processing::instruments::notes::{Octave, Pitch};
    use crate::audio_processing::processing::feature_extraction::current_feature_extractor::hybrid_extractor::HybridFeatureExtractor;
    use crate::tests::helpers::TestErrorCallback;
    use ::hound::{SampleFormat, WavSpec, WavWriter};
    use rtrb::RingBuffer;
    use std::f32::consts::PI;

    #[test]
    fn test_wav_reader_lifecycle() {
        let temp_dir = std::env::temp_dir();
        let wav_path = temp_dir.join("test_wav_reader_lifecycle.wav");

        let sample_rate = 44100;
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        {
            let mut writer = WavWriter::create(&wav_path, spec).unwrap();
            // 8192 samples of 440 Hz (A4) tone
            for i in 0..8192 {
                let t = i as f32 / sample_rate as f32;
                let sample = (2.0 * PI * 440.0 * t).sin() * 0.8;
                let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                writer.write_sample(sample_i16).unwrap();
            }
            // 4096 samples of silence
            for _ in 0..4096 {
                writer.write_sample(0i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let (prod, mut cons) = RingBuffer::<Notes>::new(64);
        let mut reader = WavReader::new(
            &wav_path,
            Instrument::Piano,
            Arc::new(TestErrorCallback),
            prod,
            -45.0,
            PitchDetectorMode::Basic,
            Arc::new(Mutex::new(GlobalSettings::default())),
        )
        .with_real_time(false);

        assert_eq!(reader.file_path, wav_path);
        assert!(!reader.is_playing.load(Ordering::Relaxed));

        // Play the WAV file through HybridFeatureExtractor
        reader.play::<HybridFeatureExtractor>().unwrap();
        reader.wait();

        // Verify notes produced
        let mut notes = Vec::new();
        while let Ok(note) = cons.pop() {
            notes.push(note);
        }

        assert!(!notes.is_empty(), "Should produce notes from WAV audio");
        let first_start = notes
            .iter()
            .find(|n| n.is_start())
            .expect("Should contain StartNote");
        assert_eq!(first_start.pitch(), Pitch::A);
        assert_eq!(first_start.octave(), Octave::O4);

        let _ = std::fs::remove_file(&wav_path);
    }

    #[test]
    fn test_stereo_wav_playback() {
        let temp_dir = std::env::temp_dir();
        let wav_path = temp_dir.join("test_wav_reader_stereo.wav");

        let sample_rate = 44100;
        let spec = WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        {
            let mut writer = WavWriter::create(&wav_path, spec).unwrap();
            // 4096 frames of stereo 440 Hz (A4)
            for i in 0..4096 {
                let t = i as f32 / sample_rate as f32;
                let sample = (2.0 * PI * 440.0 * t).sin() * 0.8;
                let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                writer.write_sample(sample_i16).unwrap(); // L
                writer.write_sample(sample_i16).unwrap(); // R
            }
            for _ in 0..2048 {
                writer.write_sample(0i16).unwrap();
                writer.write_sample(0i16).unwrap();
            }
            writer.finalize().unwrap();
        }

        let (prod, mut cons) = RingBuffer::<Notes>::new(64);
        let mut reader = WavReader::new(
            &wav_path,
            Instrument::Piano,
            Arc::new(TestErrorCallback),
            prod,
            -45.0,
            PitchDetectorMode::Basic,
            Arc::new(Mutex::new(GlobalSettings::default())),
        )
        .with_real_time(false);

        reader.play::<HybridFeatureExtractor>().unwrap();
        reader.wait();

        let mut notes = Vec::new();
        while let Ok(note) = cons.pop() {
            notes.push(note);
        }

        assert!(!notes.is_empty(), "Stereo WAV should produce notes");
        let first_start = notes
            .iter()
            .find(|n| n.is_start())
            .expect("Should contain StartNote");
        assert_eq!(first_start.pitch(), Pitch::A);
        assert_eq!(first_start.octave(), Octave::O4);

        let _ = std::fs::remove_file(&wav_path);
    }

    #[test]
    fn test_wav_reader_validation_errors() {
        let temp_dir = std::env::temp_dir();
        let empty_wav_path = temp_dir.join("test_wav_reader_empty.wav");

        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        {
            let writer = WavWriter::create(&empty_wav_path, spec).unwrap();
            writer.finalize().unwrap();
        }

        let (prod, _) = RingBuffer::<Notes>::new(64);
        let mut reader = WavReader::new(
            &empty_wav_path,
            Instrument::Piano,
            Arc::new(TestErrorCallback),
            prod,
            -45.0,
            PitchDetectorMode::Basic,
            Arc::new(Mutex::new(GlobalSettings::default())),
        );

        let res = reader.play::<HybridFeatureExtractor>();
        assert!(res.is_err(), "Empty WAV should be rejected");

        let _ = std::fs::remove_file(&empty_wav_path);
    }
}
