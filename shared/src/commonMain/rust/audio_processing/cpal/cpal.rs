/*
* Added a bunch of log infos that likely will not be used
* moved the audio engine new to just take no parameters and pass back audio engine
* added EngineSignal Enum
* Error handling:
* for most errors they will be returned as normal, but for errors that happen past the initialization of the audio engine will be accessed via the supervisor handle's join result.
* the higher up caller will need to manually reset the audio engine
* Usage:
* Call AudioEngine::new() to create a new audio engine
* If there is a problem building the stream, the audio engine is playing will become false and there will be a StreamBuildError in the thread_ error
* In that case, the higher up will have to call AudioEngine::reset() to clear the errors and allow the audio engine to be played again
* If the buffer is near full, then then the stream will exit and the user will need to reset and accomadate for errors. This process should be automatic with an alert on the user side.
* Feature extractor is now passed into the engine signal. Upon reset a new one will be created
* Todo:
** currently none
*/

use crate::constants::*;
use crate::high_pass_filter::BandPassFilter;
use crate::instruments::Instrument;
use crate::mpm::MPM;
use crate::prelude::*;
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{HeapCons, HeapRb, traits::*};
use std::sync::mpsc::{self, TryRecvError};
use std::time::Duration;

pub struct AudioEngine<T: DspCallBack, R: ErrorCallback> {
    signal_tx: Option<mpsc::Sender<EngineSignal>>,
    pub is_playing: Arc<AtomicBool>,
    error_callback: Arc<R>,
    supervisor_handle: Option<thread::JoinHandle<()>>,
    instrument: Arc<Instrument>,
    feature_extractor: Option<T>,
}

enum EngineSignal {
    Paused,
    Playing,
    Closed,
    Error { message: String }, // This type of error is always retryable
}

impl<T: DspCallBack, R: ErrorCallback> AudioEngine<T, R> {
    pub fn new(instrument: Instrument, feature_extractor: T, error_callback: Arc<R>) -> Self {
        AudioEngine {
            signal_tx: None,
            is_playing: Arc::new(AtomicBool::new(false)),
            error_callback: error_callback,
            supervisor_handle: None,
            instrument: Arc::new(instrument),
            feature_extractor: Some(feature_extractor),
        }
    }

    pub fn play(&mut self) -> Result<(), Box<dyn Error>> {
        if self.is_playing.load(Ordering::Relaxed) || self.feature_extractor.is_none() {
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
        // Build initial stream. If this fails, return Err immediately to caller
        let (initial_stream, initial_config, consumer) = Self::build_stream(&tx)?;

        let insturment = Arc::clone(&self.instrument);

        self.signal_tx = Some(tx.clone());
        self.is_playing.store(true, Ordering::Relaxed);
        let is_playing = Arc::clone(&self.is_playing); // Acts as a drop guard, notifys when dropped
        let error_callback = Arc::clone(&self.error_callback);

        let mut feature_extractor = self
            .feature_extractor
            .take()
            .expect("Feature extractor should be present");

        let handle = thread::spawn(move || {
            let _guard = DropGuard::from(is_playing);
            let mut stream: Option<Stream> = Some(initial_stream);
            let mut is_paused = false;

            // Singletons bound to the stream
            let mut config: Option<StreamConfig> = Some(initial_config);
            let mut frame = [0.0f32; FRAME_SIZE];
            let mut irr_filter_inst: Option<BandPassFilter> = Some(BandPassFilter::new(
                insturment.filter_range().hpf_cutoff_hz,
                insturment.filter_range().harmonic_ceiling_hz,
                initial_config.sample_rate,
            ));
            let mut c: Option<HeapCons<f32>> = Some(consumer);
            let mut mpm: Option<MPM> = Some(MPM::new(FRAME_SIZE / 2));

            'supervisor: loop {
                // Drain and handle all pending engine control signals
                loop {
                    match rx.try_recv() {
                        Ok(msg) => match msg {
                            EngineSignal::Closed => break 'supervisor,
                            EngineSignal::Error { message } => {
                                log::error!("Stream error: {}", message);
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
                    match Self::build_stream(&tx) {
                        Ok((st, cf, cons)) => {
                            irr_filter_inst = Some(BandPassFilter::new(
                                insturment.filter_range().hpf_cutoff_hz,
                                insturment.filter_range().harmonic_ceiling_hz,
                                cf.sample_rate,
                            ));
                            config = Some(cf);
                            stream = Some(st);
                            c = Some(cons);
                            mpm = Some(MPM::new(FRAME_SIZE / 2));
                        }
                        Err(e) => {
                            log::error!("Stream build failed: {}", e);
                            error_callback.on_error(RustError::StreamBuildError(e.to_string()));
                            break 'supervisor;
                        }
                    }
                }

                // DSP part
                if let (Some(_), Some(cfg), Some(cons)) = (&stream, &config, &mut c) {
                    let mut processed = false;

                    const NEAR_BUFFER_OVERFILL: usize = RINGBUF_CAPACITY - FRAME_SIZE;
                    if cons.occupied_len() >= NEAR_BUFFER_OVERFILL {
                        let excess = cons.occupied_len() - FRAME_SIZE;
                        cons.skip(excess);
                        error_callback.on_error(RustError::BufferOverfill);
                        break 'supervisor;
                    }

                    if cons.occupied_len() >= FRAME_SIZE {
                        let (first, second) = cons.as_slices();
                        if first.len() >= FRAME_SIZE {
                            frame.copy_from_slice(&first[..FRAME_SIZE]);
                        } else {
                            let first_len = first.len();
                            frame[..first_len].copy_from_slice(first);
                            frame[first_len..FRAME_SIZE]
                                .copy_from_slice(&second[..FRAME_SIZE - first_len]);
                        }

                        if let (Some(filter), Some(mpm)) = (&mut irr_filter_inst, &mut mpm) {
                            feature_extractor.dsp_callback(CallBackParameters {
                                buffer: &frame,
                                cfg,
                                filter,
                                instrument: &insturment,
                                mpm,
                            });
                        }

                        processed = true;

                        cons.skip(HOP_SIZE);
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

    fn build_stream(
        signal_tx: &mpsc::Sender<EngineSignal>,
    ) -> Result<(Stream, StreamConfig, HeapCons<f32>), Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No mic")?;

        let supported_configs = device.supported_input_configs()?.collect::<Vec<_>>();

        let preferred_rates = PREFERRED_RATES;
        let mut selected_format: Option<SampleFormat> = None;
        let mut selected_config: Option<StreamConfig> = None;

        'outer: for &target_format in &[SampleFormat::F32, SampleFormat::I32, SampleFormat::I16] {
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
                            break 'outer;
                        }
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

        log::info!(
            "Starting audio input stream with format {:?} and config: {:?}",
            sample_format,
            config
        );

        let err_signal_tx = signal_tx.clone();
        let err_fn = move |err: cpal::Error| {
            log::error!("Audio input stream error: {}", err);

            let _ = err_signal_tx.send(EngineSignal::Error {
                message: err.to_string(),
            });
        };
        let rb = HeapRb::<f32>::new(RINGBUF_CAPACITY);
        let (mut prod, cons) = rb.split();

        let channels = config.channels;
        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| match channels {
                    1 => {
                        let _ = prod.push_slice(data);
                    }
                    2 => {
                        let _ = prod.push_iter(data.chunks_exact(2).map(|c| (c[0] + c[1]) * 0.5));
                    }
                    _ => {
                        let ch = channels as usize;
                        let _ = prod.push_iter(data.chunks_exact(ch).map(|c| c[0]));
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
                            let _ = prod.push_iter(data.iter().map(|&s| s as f32 * NORM));
                        }
                        2 => {
                            let _ = prod.push_iter(
                                data.chunks_exact(2)
                                    .map(|c| (c[0] as f32 + c[1] as f32) * (0.5 * NORM)),
                            );
                        }
                        _ => {
                            let ch = channels as usize;
                            let _ =
                                prod.push_iter(data.chunks_exact(ch).map(|c| c[0] as f32 * NORM));
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
                            let _ = prod.push_iter(data.iter().map(|&s| s as f32 * NORM));
                        }
                        2 => {
                            let _ = prod.push_iter(
                                data.chunks_exact(2)
                                    .map(|c| (c[0] as f32 + c[1] as f32) * (0.5 * NORM)),
                            );
                        }
                        _ => {
                            let ch = channels as usize;
                            let _ =
                                prod.push_iter(data.chunks_exact(ch).map(|c| c[0] as f32 * NORM));
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

    pub fn reset(&mut self, feature_extractor: T) {
        self.end();
        self.signal_tx = None;
        self.is_playing.store(false, Ordering::Relaxed);
        self.supervisor_handle = None;
        self.feature_extractor = Some(feature_extractor);
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

impl<T: DspCallBack, R: ErrorCallback> Drop for AudioEngine<T, R> {
    fn drop(&mut self) {
        self.end()
    }
}
