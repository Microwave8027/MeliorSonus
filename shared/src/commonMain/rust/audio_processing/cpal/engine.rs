/*
 * Audio Engine Implementation using CPAL
 *
 * Usage:
 * Call AudioEngine::new() to create a new audio engine
 * If there is a problem building the stream, the audio engine is_playing will become false and an error callback will fire.
 * The feature extractor state is tied to the build stream
 */

use crate::audio_processing::PitchDetectorMode;
use crate::audio_processing::dsp::{CallBackParameters, DspCallBack};
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::processing::functions::filters::band_pass_filter::BandPassFilter;
use crate::audio_processing::processing::functions::pitch::mpm::MPM;
use crate::constants::*;
use crate::audio_processing::Notes;
use crate::utils::error_callback::ErrorCallback;
use crate::utils::errors::{AudioEngineError, RustError};
use crate::utils::global_settings::GlobalSettings;
use crate::utils::guard::DropGuard;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

pub struct AudioEngine {
    signal_tx: Option<mpsc::Sender<EngineSignal>>,
    pub is_playing: Arc<AtomicBool>,
    error_callback: Arc<dyn ErrorCallback>,
    supervisor_handle: Option<thread::JoinHandle<()>>,
    instrument: Arc<Instrument>,
    rt_rb_prod: Option<Producer<Notes>>, //used for notes not buffers
    silence_threshold: f32,
    pitch_detector_mode: PitchDetectorMode,
    pub audio_device_name: Option<String>,
    pub global_settings: Arc<Mutex<GlobalSettings>>,
}

enum EngineSignal {
    Paused,
    Playing,
    Closed,
    SwitchDevice { device_name: Option<String> },
    Error { message: String }, // This type of error is always retryable
}

impl AudioEngine {
    pub fn new(
        instrument: Instrument,
        error_callback: Arc<dyn ErrorCallback>,
        rtrb_prod: Producer<Notes>,
        silence_threshold: f32,
        pitch_detector_mode: PitchDetectorMode,
        global_settings: Arc<Mutex<GlobalSettings>>,
    ) -> Self {
        AudioEngine {
            signal_tx: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            error_callback,
            supervisor_handle: None,
            instrument: Arc::new(instrument),
            rt_rb_prod: Some(rtrb_prod),
            silence_threshold,
            pitch_detector_mode: pitch_detector_mode,
            audio_device_name: None,
            global_settings,
        }
    }

    pub fn set_audio_device(&mut self, device_name: Option<String>) {
        self.audio_device_name = device_name.clone();
        if let Some(tx) = &self.signal_tx {
            let _ = tx.send(EngineSignal::SwitchDevice { device_name });
        }
    }

    pub fn with_audio_device(mut self, device_name: Option<String>) -> Self {
        self.audio_device_name = device_name;
        self
    }

    pub fn play<T: DspCallBack>(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_playing.load(Ordering::Relaxed) {
            return Err("Reset Audio Engine and read errors before playing again".into());
        }
        // Checks if the thread is already running
        if self
            .supervisor_handle
            .as_ref()
            .is_some_and(|h| !h.is_finished())
        {
            return Err("Audio engine is already playing".into());
        }

        let (tx, rx) = mpsc::channel();
        let target_device_name = self.audio_device_name.clone();
        // Build initial stream. If this fails, return Err immediately to caller
        let (initial_stream, initial_config, consumer) =
            Self::build_stream(&tx, target_device_name.as_deref())?;

        let instrument = Arc::clone(&self.instrument);

        self.signal_tx = Some(tx.clone());
        self.is_playing.store(true, Ordering::Relaxed);
        let is_playing = Arc::clone(&self.is_playing); // Acts as a drop guard, notifies when dropped
        let error_callback = Arc::clone(&self.error_callback);
        let threshold = self.silence_threshold;
        let mode = self.pitch_detector_mode;
        let global_settings = Arc::clone(&self.global_settings);

        let Some(rb_prod) = self.rt_rb_prod.take() else {
            return Err("No consumer was supplied, please reset the audio engine with a producer before trying again".into());
        };

        let handle = thread::spawn(move || {
            crate::utils::enable_flush_to_zero();
            let _guard = DropGuard::from(is_playing);
            let mut target_device_name = target_device_name;
            let mut stream: Option<Stream> = Some(initial_stream);
            let mut is_paused = false;

            // Singletons bound to the stream
            let mut config: Option<StreamConfig> = Some(initial_config);
            let mut frame = [0.0f32; FRAME_SIZE];
            let mut irr_filter_inst: Option<BandPassFilter> = Some(BandPassFilter::new(
                instrument.filter_range().hpf_cutoff_hz,
                instrument.filter_range().harmonic_ceiling_hz,
                initial_config.sample_rate,
            ));
            let mut c: Option<Consumer<f32>> = Some(consumer);
            let mut mpm: Option<MPM> = Some(MPM::new(FRAME_SIZE / 2));
            let mut feature_extractor: Option<T> = match T::new(
                rb_prod,
                config.as_ref().unwrap().sample_rate,
                threshold,
                mode,
                global_settings,
            ) {
                Ok(v) => Some(v),
                Err(err) => {
                    error_callback.on_error(RustError::FeatureExtactorBuildError(err.to_string()));
                    return;
                }
            };

            'supervisor: loop {
                // Drain and handle all pending engine control signals
                loop {
                    match rx.try_recv() {
                        Ok(msg) => match msg {
                            EngineSignal::Closed => break 'supervisor,
                            EngineSignal::SwitchDevice { device_name } => {
                                // log::info!("Switching audio input device to: {:?}", device_name);
                                target_device_name = device_name;
                                stream = None;
                                config = None;
                                c = None;
                                continue 'supervisor;
                            }
                            EngineSignal::Error { message } => {
                                // log::error!("Stream error: {}", message);
                                error_callback.on_error(RustError::AudioEngineError(
                                    AudioEngineError::StreamBuildError(message),
                                ));
                                stream = None;
                                config = None;
                                c = None;
                                thread::sleep(Duration::from_millis(300)); // Debounce before reconnecting
                                continue 'supervisor;
                            }
                            EngineSignal::Paused => {
                                is_paused = true;
                                if let Some(ref s) = stream {
                                    let _ = s.pause();
                                }
                            }
                            EngineSignal::Playing => {
                                is_paused = false;
                                if let Some(ref s) = stream {
                                    let _ = s.play();
                                }
                            }
                        },
                        Err(TryRecvError::Disconnected) => break 'supervisor,
                        Err(TryRecvError::Empty) => break,
                    }
                }

                // If paused, sleep briefly and loop to wait for resume or close
                if is_paused {
                    thread::sleep(Duration::from_millis(10));
                    continue 'supervisor;
                }

                // Rebuilds the stream if it was destroyed by a runtime error
                if stream.is_none() {
                    match Self::build_stream(&tx, target_device_name.as_deref()) {
                        Ok((st, cf, cons)) => {
                            irr_filter_inst = Some(BandPassFilter::new(
                                instrument.filter_range().hpf_cutoff_hz,
                                instrument.filter_range().harmonic_ceiling_hz,
                                cf.sample_rate,
                            ));
                            config = Some(cf);
                            stream = Some(st);
                            c = Some(cons);
                            mpm = Some(MPM::new(FRAME_SIZE / 2));
                        }
                        Err(e) => {
                            // log::error!("Stream build failed: {}", e);
                            error_callback.on_error(RustError::AudioEngineError(
                                AudioEngineError::StreamBuildError(e.to_string()),
                            ));
                            break 'supervisor;
                        }
                    }
                }

                // DSP part
                if let (Some(_), Some(cfg), Some(cons), Some(fe)) =
                    (&stream, &config, &mut c, &mut feature_extractor)
                {
                    let mut processed = false;

                    const NEAR_BUFFER_OVERFILL: usize = RINGBUF_CAPACITY - FRAME_SIZE;
                    let occupied = cons.slots();
                    if occupied >= NEAR_BUFFER_OVERFILL {
                        let excess = occupied - FRAME_SIZE;
                        if let Ok(chunk) = cons.read_chunk(excess) {
                            chunk.commit_all();
                        }
                        error_callback.on_error(RustError::AudioEngineError(
                            AudioEngineError::BufferOverfill,
                        ));
                    }

                    if cons.slots() >= FRAME_SIZE {
                        if let Ok(chunk) = cons.read_chunk(FRAME_SIZE) {
                            let (first, second) = chunk.as_slices();
                            if first.len() >= FRAME_SIZE {
                                frame.copy_from_slice(&first[..FRAME_SIZE]);
                            } else {
                                let first_len = first.len();
                                frame[..first_len].copy_from_slice(first);
                                frame[first_len..FRAME_SIZE]
                                    .copy_from_slice(&second[..FRAME_SIZE - first_len]);
                            }

                            if let (Some(filter), Some(mpm)) = (&mut irr_filter_inst, &mut mpm) {
                                fe.dsp_callback(CallBackParameters {
                                    buffer: &frame,
                                    cfg,
                                    filter,
                                    instrument: &instrument,
                                    mpm,
                                });
                            }

                            processed = true;

                            chunk.commit(HOP_SIZE);
                        }
                    }

                    if !processed {
                        thread::sleep(Duration::from_millis(5));
                    }
                }
            }
        });

        self.supervisor_handle = Some(handle);
        Ok(())
    }

    pub fn get_device_name(device: &cpal::Device) -> String {
        if let Ok(desc) = device.description() {
            desc.name().to_string()
        } else {
            format!("{:?}", device.id())
        }
    }

    pub fn list_input_devices() -> Result<Vec<(usize, String, bool)>, Box<dyn Error>> {
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .map(|d| Self::get_device_name(&d));
        let devices = host.input_devices()?;
        let mut list = Vec::new();
        for (i, dev) in devices.enumerate() {
            let name = Self::get_device_name(&dev);
            let is_default = default_name.as_ref().map_or(false, |dn| dn == &name);
            list.push((i, name, is_default));
        }
        Ok(list)
    }

    pub fn find_input_device(
        host: &cpal::Host,
        target: &str,
    ) -> Result<cpal::Device, Box<dyn Error>> {
        if target.eq_ignore_ascii_case("default") {
            return host
                .default_input_device()
                .ok_or_else(|| "No default microphone/input device found".into());
        }

        let devices = host.input_devices()?.collect::<Vec<_>>();

        if let Ok(idx) = target.parse::<usize>() {
            if let Some(dev) = devices.get(idx) {
                return Ok(dev.clone());
            }
        }

        for dev in &devices {
            let name = Self::get_device_name(dev);
            if name.eq_ignore_ascii_case(target) {
                return Ok(dev.clone());
            }
        }

        let target_lower = target.to_lowercase();
        for dev in &devices {
            let name = Self::get_device_name(dev);
            if name.to_lowercase().contains(&target_lower) {
                return Ok(dev.clone());
            }
        }

        let available = devices
            .iter()
            .enumerate()
            .map(|(i, d)| format!("  [{}] {}", i, Self::get_device_name(d)))
            .collect::<Vec<_>>()
            .join("\n");

        Err(format!(
            "Input device '{}' not found.\nAvailable input devices:\n{}",
            target, available
        )
        .into())
    }

    fn build_stream(
        signal_tx: &mpsc::Sender<EngineSignal>,
        device_name: Option<&str>,
    ) -> Result<(Stream, StreamConfig, Consumer<f32>), Box<dyn Error>> {
        let host = cpal::default_host();
        let device = match device_name {
            Some(target) => Self::find_input_device(&host, target)?,
            None => host
                .default_input_device()
                .ok_or_else(|| "No default audio input device found".to_string())?,
        };

        /*log::info!(
            "Selected audio input device: '{}'",
            Self::get_device_name(&device)
        );*/

        let supported_configs = device.supported_input_configs()?.collect::<Vec<_>>();

        let preferred_rates = PREFERRED_RATES;
        let mut selected_format: Option<SampleFormat> = None;
        let mut selected_config: Option<StreamConfig> = None;

        let formats = [
            SampleFormat::F32,
            SampleFormat::I32,
            SampleFormat::I16,
            SampleFormat::U16,
        ];

        // Try preferred sample rate with mono or stereo
        'pass1: for &target_format in &formats {
            for range in &supported_configs {
                if range.sample_format() == target_format
                    && (range.channels() == 1 || range.channels() == 2)
                {
                    let min_rate = range.min_sample_rate();
                    let max_rate = range.max_sample_rate();

                    for &rate in &preferred_rates {
                        if rate >= min_rate && rate <= max_rate {
                            selected_format = Some(target_format);
                            selected_config = Some(StreamConfig {
                                channels: range.channels(),
                                sample_rate: rate,
                                buffer_size: cpal::BufferSize::Default,
                            });
                            break 'pass1;
                        }
                    }
                }
            }
        }

        // If no 1/2 channel found with preferred rates, try any channel count with preferred rates
        if selected_config.is_none() {
            'pass2: for &target_format in &formats {
                for range in &supported_configs {
                    if range.sample_format() == target_format {
                        let min_rate = range.min_sample_rate();
                        let max_rate = range.max_sample_rate();

                        for &rate in &preferred_rates {
                            if rate >= min_rate && rate <= max_rate {
                                selected_format = Some(target_format);
                                selected_config = Some(StreamConfig {
                                    channels: range.channels(),
                                    sample_rate: rate,
                                    buffer_size: cpal::BufferSize::Default,
                                });
                                break 'pass2;
                            }
                        }
                    }
                }
            }
        }

        // Try 1 or 2 channels with any supported sample rate
        if selected_config.is_none() {
            'pass3: for &target_format in &formats {
                for range in &supported_configs {
                    if range.sample_format() == target_format
                        && (range.channels() == 1 || range.channels() == 2)
                    {
                        selected_format = Some(target_format);
                        selected_config = Some(StreamConfig {
                            channels: range.channels(),
                            sample_rate: range.max_sample_rate(),
                            buffer_size: cpal::BufferSize::Default,
                        });
                        break 'pass3;
                    }
                }
            }
        }

        let (sample_format, config) = match (selected_format, selected_config) {
            (Some(fmt), Some(cfg)) => (fmt, cfg),
            _ => {
                let default_cfg = device.default_input_config()?;
                (default_cfg.sample_format(), default_cfg.into())
            }
        };

        /* log::info!(
            "Starting audio input stream with format {:?} and config: {:?}",
            sample_format,
            config
        );*/

        let err_signal_tx = signal_tx.clone();
        let err_fn = move |err: cpal::Error| {
            // log::error!("Audio input stream error: {}", err);

            let _ = err_signal_tx.send(EngineSignal::Error {
                message: err.to_string(),
            });
        };
        let (mut prod, cons) = RingBuffer::<f32>::new(RINGBUF_CAPACITY);

        let channels = config.channels;
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| match channels {
                    1 => {
                        for &s in data {
                            let _ = prod.push(s);
                        }
                    }
                    2 => {
                        for c in data.chunks_exact(2) {
                            let _ = prod.push((c[0] + c[1]) * 0.5);
                        }
                    }
                    _ => {
                        let ch = channels as usize;
                        let inv_ch = 1.0 / ch as f32;
                        for c in data.chunks_exact(ch) {
                            let sum: f32 = c.iter().sum();
                            let _ = prod.push(sum * inv_ch);
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::I32 => device.build_input_stream(
                config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    const NORM: f32 = 1.0 / 2147483648.0;
                    match channels {
                        1 => {
                            for &s in data {
                                let _ = prod.push(s as f32 * NORM);
                            }
                        }
                        2 => {
                            for c in data.chunks_exact(2) {
                                let _ = prod.push((c[0] as f32 + c[1] as f32) * (0.5 * NORM));
                            }
                        }
                        _ => {
                            let ch = channels as usize;
                            let inv_ch = NORM / ch as f32;
                            for c in data.chunks_exact(ch) {
                                let sum: f32 = c.iter().map(|&x| x as f32).sum();
                                let _ = prod.push(sum * inv_ch);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    const NORM: f32 = 1.0 / 32768.0;
                    match channels {
                        1 => {
                            for &s in data {
                                let _ = prod.push(s as f32 * NORM);
                            }
                        }
                        2 => {
                            for c in data.chunks_exact(2) {
                                let _ = prod.push((c[0] as f32 + c[1] as f32) * (0.5 * NORM));
                            }
                        }
                        _ => {
                            let ch = channels as usize;
                            let inv_ch = NORM / ch as f32;
                            for c in data.chunks_exact(ch) {
                                let sum: f32 = c.iter().map(|&x| x as f32).sum();
                                let _ = prod.push(sum * inv_ch);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                config,
                move |data: &[u16], _: &cpal::InputCallbackInfo| {
                    const NORM: f32 = 1.0 / 32768.0;
                    match channels {
                        1 => {
                            for &s in data {
                                let _ = prod.push((s as f32 - 32768.0) * NORM);
                            }
                        }
                        2 => {
                            for c in data.chunks_exact(2) {
                                let s0 = (c[0] as f32 - 32768.0) * NORM;
                                let s1 = (c[1] as f32 - 32768.0) * NORM;
                                let _ = prod.push((s0 + s1) * 0.5);
                            }
                        }
                        _ => {
                            let ch = channels as usize;
                            let inv_ch = NORM / ch as f32;
                            for c in data.chunks_exact(ch) {
                                let sum: f32 = c.iter().map(|&x| x as f32 - 32768.0).sum();
                                let _ = prod.push(sum * inv_ch);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?,
            unsupported => {
                return Err(format!("Unsupported sample format: {:?}", unsupported).into());
            }
        };

        stream.play()?;

        Ok((stream, config, cons))
    }

    pub fn reset(&mut self, rtrb_prod: Producer<Notes>) {
        self.end();
        self.signal_tx = None;
        self.is_playing.store(false, Ordering::Relaxed);
        self.supervisor_handle = None;
        self.rt_rb_prod = Some(rtrb_prod);
    }

    pub fn pause(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(tx) = &self.signal_tx {
            self.is_playing.store(false, Ordering::Relaxed);
            tx.send(EngineSignal::Paused)?;
            Ok(())
        } else {
            Err("Audio not initialized".into())
        }
    }

    pub fn resume(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(tx) = &self.signal_tx {
            self.is_playing.store(true, Ordering::Relaxed);
            tx.send(EngineSignal::Playing)?;
            Ok(())
        } else {
            Err("Audio not initialized".into())
        }
    }

    pub fn end(&mut self) {
        if let Some(tx) = &self.signal_tx {
            let _ = tx.send(EngineSignal::Closed);
        }
        if let Some(handle) = self.supervisor_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.end()
    }
}
