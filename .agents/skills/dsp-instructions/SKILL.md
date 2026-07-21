---
name: audio-dsp-guide
description: Guide for writing real-time audio analysis and DSP logic in Kotlin.
---
# Core DSP Rules
- **No Allocation in the Audio Loop:** Never allocate memory, instantiate classes, or initialize objects inside real-time audio processing loops or audio capture callbacks. It triggers the Garbage Collector and causes audible stuttering.
- **Threading:** Real-time audio data must be processed entirely on high-priority background threads using custom Coroutine dispatchers. Never route raw mic buffers through the Main dispatcher.
- **FFT Size:** For vocal pitch detection and music tracking, default to an FFT size of 2048 or 4096 samples at a 44.1kHz sampling rate to ensure acceptable frequency resolution.- **Windowing:** Apply a Hann or Hamming window to audio frames before performing FFT to reduce spectral leakage.
- **Data Types:** Use `FloatArray` for audio buffers to maintain precision and compatibility with most DSP libraries and hardware abstractions.
