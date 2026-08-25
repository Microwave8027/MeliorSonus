/*
* The dsp is not a a noise cancellation machine, it simply just processes audio. Here is the general layout:
Hardware Buffer (&[u8])
       │
       ▼ (Decode PCM)
Raw Floats (&[f32])
       │
       ▼
[High-Pass Filter (>30 Hz)] ──► Strips DC / Subsonic Rumble
       │
       ▼
┌──────────────────────────────────────────────────────────┐
│ Time-Domain Stage (Low CPU, Zero Latency)                │
│  ├─ RMS & Dynamic Level Calculation                      │
│  ├─ Attack/Transient Slope Detection (Staccato/Accent)   │
│  └─ YIN / MPM Pitch Tracker (f0 -> MIDI Note & Cents)    │
└──────────────────────────────┬───────────────────────────┘
                               │
                               ▼ (Overlap Framing & Windowing, noise cancellation algorithm)
┌──────────────────────────────────────────────────────────┐
│ Frequency-Domain Stage (FFT)                             │
│  ├─ Compute Magnitude Spectrum |X[k]|                    │
│  ├─ Spectral Denoising (Spectral Subtraction / Wiener)   │
│  ├─ Harmonics-to-Noise Ratio (HNR) & Spectral Flatness   │
│  ├─ Spectral Centroid (Tone Brightness)                  │
└──────────────────────────────────────────────────────────┘
* In fact, noise cancellation might not even be used
* The entire workflow is done on one threadw(its cpu bound)
*/

use crate::{
    constants::*, high_pass_filter::BandPassFilter, instruments::Instrument, mpm::MPM, prelude::*,
};

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

#[allow(dead_code)]
pub struct Dsp<T: ErrorCallback> {
    instrument: Instrument,
    silence_threshold: f32,
    rb_cons: Option<HeapCons<Note>>,
    error_callback: Arc<T>,
    pub audio_engine: AudioEngine<DspFeatureExtractor, T>,
}

impl<T: ErrorCallback> Dsp<T> {
    pub fn new(instrument: Instrument, silence_threshold: f32, error_callback: T) -> Self {
        let rb = HeapRb::<Note>::new(NOTE_RINGBUF_CAPACITY);
        let (prod, cons) = rb.split();
        let feature_extractor = DspFeatureExtractor::new(
            prod,
            NoteFeatureExtractorImpl::new(silence_threshold),
            PolyphonicFeatureExtractorImpl::new(),
            silence_threshold,
        );
        let arc_err_callback = Arc::new(error_callback);
        let audio_engine =
            AudioEngine::new(instrument, feature_extractor, Arc::clone(&arc_err_callback));

        Self {
            instrument,
            silence_threshold,
            rb_cons: Some(cons),
            error_callback: arc_err_callback,
            audio_engine,
        }
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
}
