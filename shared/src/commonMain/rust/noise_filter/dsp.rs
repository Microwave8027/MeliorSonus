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

use crate::{high_pass_filter::BandPassFilter, prelude::*};

pub trait DspCallBack: Send + 'static {
    fn dsp_callback(
        &self,
        buffer: [f32; FRAME_SIZE],
        cfg: &StreamConfig,
        filter: &mut BandPassFilter,
    ) -> ();
}
