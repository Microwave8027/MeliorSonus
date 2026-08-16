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
│  └─ Push Slices to 60 FPS Visualizer                     │
└──────────────────────────────────────────────────────────┘
* In fact, noise cancellation might not even be used
* The entire workflow is done on one threadw(its cpu bound)
*/

use crate::{high_pass_filter::BandPassFilter, instruments::Instrument, mpm::MPM, prelude::*};

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
    ) -> () {
    }
}
