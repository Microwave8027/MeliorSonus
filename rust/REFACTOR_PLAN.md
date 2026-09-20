# MeliorSonus Audio Engine Refactor Plan: Neural Note Tracking + MPM Real-Time Stream + Hybrid Acoustic Analytics

**Document Version:** 2.2.0  
**Target Architecture:** Rust DSP Core (`rubato`, `realfft`, `biquad`, `pitch-detection`, `rtrb`) + LiteRT C FFI via `bindgen` (GPU/CoreML/XNNPACK hardware-accelerated inference) with `tract-onnx` CPU fallback + Spotify Basic Pitch + MPM (Instant Transients & Pitch Fast-Path) + MeliorSonus Acoustic Analytics + Non-Real-Time Score Following (`musicxml`, `fastdtw`, `symphonia`, `midly`)  
**Supported Platforms:** Android (ARM64/x86_64), iOS (ARM64), Desktop (macOS/Windows/Linux)

---

## 🤖 AI Implementation Guide & Execution Directives

> [!IMPORTANT]
> **Directive for AI Assistants & Autonomous Agents:**
> When prompted to implement, refactor, or debug audio procedures in this codebase, **strictly adhere to the execution tags** below:
> 
> * `[REAL-TIME SAFE: ZERO ALLOCATION]` — Code executing on audio acquisition or high-priority DSP loops.
>   - **Forbidden:** No heap allocations (`Vec`, `Box`, `String`, `format!`), no blocking synchronization (`Mutex`, `RwLock`, `std::sync::mpsc`), no I/O, no system calls.
>   - **Required:** Use fixed-size stack arrays (`[f32; N]`), `ArrayVec`, or pre-allocated scratch buffers. All cross-thread transfers must use lock-free, wait-free SPSC ringbuffers (`rtrb`).
> * `[REAL-TIME SAFE: MPM DSP FAST-PATH]` — Native sample-rate (44.1k/48k) processing using pure Rust MPM (`pitch-detection`) for instant pitch tracking for live tuner UI.
> * `[BACKGROUND DSP WORKER]` — High-priority processing thread draining audio buffers from the lock-free ring buffer (resampling to 22.05kHz via `rubato`, neural inference via `tract-onnx`, note segmentation).
> * `[OFFLINE / NON-REAL-TIME WORKER]` — Asynchronous tasks running on standard background coroutines or worker threads (MusicXML parsing, score alignment via FastDTW, audio decoding via Symphonia). Dynamic allocation and standard error handling are permitted here.
> * `[PEDAGOGICAL CORE: KEEP CUSTOM]` — Proprietary MeliorSonus IP (`is_mashed`, `is_flat`, articulation classifiers, card feedback). Do **not** replace these domain heuristics with generic external libraries.

---

## 1. Executive Summary & Problem Statement

### 1.1 The Current Problem
The legacy MeliorSonus audio processing pipeline relies on a hand-crafted real-time DSP state machine:
* **McLeod Pitch Method (MPM) & Peak Picking:** Requires manual tuning of clarity thresholds, power thresholds, and octave jump tolerances per instrument.
* **Envelope State Machine (`feature_extractor_state.rs`):** Employs complex heuristic gates (`Rise`, `Peak`, `Decay`, `Idle`) to distinguish between repeated attacks (staccatos, brass tonguing), vibrato oscillations, breath dropouts, and volume swells.
* **Missing Resampling Pipeline:** Hardware microphones sample at 44.1 kHz or 48.0 kHz, but neural models require 22.05 kHz.
* **Transient Latency Trade-off:** Neural models operating on 512-sample hops (~23.2 ms) introduce a small temporal delay for instant visual tuner feedback and sub-millisecond attack transients.

### 1.2 The Proposed Solution: Dual-Stream Hybrid Pipeline
Combine **Pure Rust MPM Fast-Path** (running at native 44.1k/48k sample rates for live tuner tracking) with **Spotify's Basic Pitch** running via **LiteRT C FFI** (primary, hardware-accelerated via GPU/CoreML/XNNPACK delegates) or **`tract-onnx`** (CPU fallback for desktop/unsupported targets), while **retaining 100% of MeliorSonus' rich acoustic and pedagogical analytics**:

```
                    ┌──────────────────────────────────────────────┐
                    │      Incoming Audio Frames (Mic Stream)      │
                    │         (Native 44.1 kHz / 48.0 kHz)         │
                    └──────────────────────┬───────────────────────┘
                                           │ [REAL-TIME SAFE: CPAL Callback]
                                           ▼
                    ┌──────────────────────────────────────────────┐
                    │      rtrb (Lock-Free SPSC Ring Buffer)       │
                    └──────────────────────┬───────────────────────┘
                                           │ (Cross-Thread Transfer)
                    ┌──────────────────────┴───────────────────────┐
                    │                                              │
                    ▼ [REAL-TIME MPM FAST-PATH]                   ▼ [BACKGROUND DSP WORKER]
     ┌─────────────────────────────┐               ┌─────────────────────────────┐
     │      pitch-detection        │               │           rubato            │
     │  (Native Sample Rate MPM)   │               │ (Resample 44.1/48k -> 22.05k│
     ├─────────────────────────────┤               └──────────────┬──────────────┘
     │ • Instant Pitch (60 FPS     │                              │
     │   Tuner Needle / Live Meter)│                              ▼
     │ • RealFFT Spectral Centroid │               ┌─────────────────────────────┐
     │ • Biquad Band-Pass Filter   │               │   NeuralTranscriber Trait   │
     └──────────────┬──────────────┘               │   (Runtime Backend Select)  │
                    │                              ├─────────────┬───────────────┤
                    │                              │             │               │
                    │                              ▼             ▼              │
                    │               ┌──────────────────┐ ┌──────────────────┐    │
                    │               │ LiteRtTranscriber│ │TractBasicPitch-  │    │
                    │               │  (Primary)       │ │Model (Fallback)  │    │
                    │               │                  │ │                  │    │
                    │               │ LiteRT C FFI via │ │ Pure Rust ONNX   │    │
                    │               │ bindgen:         │ │ (tract-onnx)     │    │
                    │               │ • GPU Delegate   │ │ • CPU SIMD only  │    │
                    │               │ • CoreML (iOS)   │ │ • No C deps      │    │
                    │               │ • XNNPACK (CPU)  │ │ • Desktop/debug  │    │
                    │               └──────────────────┘ └──────────────────┘    │
                    │                               │                            │
                    │                               └──────────────┬─────────────┘
                    │                                              │
                    │                               ┌──────────────┴──────────────┐
                    │                               │  Spotify Basic Pitch Output │
                    │                               │  • onsets[88], frames[88]   │
                    │                               │  • contours[264]            │
                    │                               └──────────────┬──────────────┘
                    │                                              │
                    └──────────────────────┬───────────────────────┘
                                           ▼
                    ┌──────────────────────────────────────────────┐
                    │       MeliorSonus Acoustic Analytics         │
                    │  (realfft Centroid, Crest Factor, Sub-Thump) │
                    └──────────────────────┬───────────────────────┘
                                           ▼
                    ┌──────────────────────────────────────────────┐
                    │       Unified RecordNote -> EndNote          │
                    │   (Rock-solid Notes + Rich Pedagogy)         │
                    └──────────────────────┬───────────────────────┘
                                           │ [NON-REAL-TIME: Post-Processing]
                                           ▼
                    ┌──────────────────────────────────────────────┐
                    │   MusicXML + FastDTW Score-Following Engine  │
                    │ (Align Performance with Reference Sheet Score)│
                    └──────────────────────────────────────────────┘
```

---

## 2. Comprehensive Dependency Breakdown & Domain Classification

### 2.1 Real-Time Audio Extraction & Neural DSP Stack

| Crate | Target Version | Domain Tag | Role & Replacement Target |
| :--- | :--- | :--- | :--- |
| **`cpal`** | `0.18.1` | `[REAL-TIME SAFE]` | Hardware microphone stream capture and audio device host abstraction. |
| **`rtrb`** | `0.3` | `[REAL-TIME SAFE]` | **Replaces `ringbuf::HeapRb`**. Verified lock-free, wait-free SPSC ring buffer for passing PCM chunks from audio callback to DSP worker without locking. |
| **`pitch-detection`** | `0.3` | `[REAL-TIME SAFE: MPM]` | **Dual-Stream Fast Path**. Instantaneous MPM pitch tracking and clarity calculation for live 60 FPS tuner UI. |
| **`realfft`** | `3.4` | `[REAL-TIME SAFE]` | **Replaces scalar `real_fft.rs`**. SIMD-accelerated (ARM NEON / AVX2) real-to-complex FFT with pre-allocated scratch buffers. 10–20x faster than hand-rolled Cooley-Tukey. |
| **`biquad`** | `0.4` | `[REAL-TIME SAFE]` | **Replaces custom `band_pass_filter.rs`**. Zero-allocation `#![no_std]` Direct Form 2 Transposed biquad filters for DC cut and instrument band-pass. |
| **`rubato`** | `0.16` | `[BACKGROUND DSP WORKER]` | **New Core Primitive**. Real-time safe Sinc/FFT resampler to convert native mic sample rates (44.1 kHz / 48 kHz) to 22.05 kHz required by Basic Pitch. |
| **`tract-onnx`** | `0.21` | `[BACKGROUND DSP WORKER]` | **CPU Fallback Backend**. Pure-Rust ONNX inference runtime. Feature-gated (`#[cfg(feature = "tract-backend")]`) fallback for desktop targets and development builds without vendored LiteRT static libraries. Zero C/C++ dependencies. |

### 2.1b LiteRT C FFI — Primary Hardware-Accelerated Neural Inference Stack

> [!IMPORTANT]
> LiteRT (formerly TensorFlow Lite) is the **primary inference backend** for mobile targets (Android ARM64, iOS ARM64).
> It is integrated via **C FFI** using `bindgen` to generate Rust bindings from the LiteRT C API headers (`c_api.h`).
> Prebuilt static libraries (`.a`) are **vendored per target architecture** and linked via `build.rs`.

| Component | Version / Source | Domain Tag | Role & Integration Details |
| :--- | :--- | :--- | :--- |
| **LiteRT C API** | LiteRT 2.x (`google-ai-edge/litert`) | `[BACKGROUND DSP WORKER]` | **Primary Neural Inference Backend**. Executes the `.tflite` quantized Spotify Basic Pitch model with hardware delegate acceleration (GPU, CoreML, XNNPACK). Linked as a vendored static library (`.a`) per target architecture via `build.rs`. |
| **`bindgen`** | `0.71` | `[BUILD DEPENDENCY]` | Generates raw Rust FFI bindings from LiteRT `c_api.h` headers at compile time in `build.rs`. Produces `litert_bindings.rs` in `OUT_DIR`. |
| **Vendored Static Libraries** | Per-architecture `.a` files | `[BUILD ARTIFACT]` | Prebuilt `libtensorflowlite_c.a` for each target: `android-arm64`, `android-arm32`, `ios-arm64`, `ios-arm64-sim`. Built from source via CMake (Android) or Bazel (iOS) and committed to `native-libs/litert/`. |
| **GPU Delegate** | Bundled with LiteRT | `[BACKGROUND DSP WORKER]` | OpenGL ES / Metal GPU acceleration for floating-point models. ~1-3ms inference on mobile GPUs. |
| **CoreML Delegate** | Bundled with LiteRT | `[BACKGROUND DSP WORKER]` | iOS-only. Routes inference to the Apple Neural Engine (ANE) on A12+ chips. ~0.5-2ms inference. Requires linking `CoreML.framework` and `Accelerate.framework`. |
| **XNNPACK Delegate** | Bundled with LiteRT | `[BACKGROUND DSP WORKER]` | Optimized CPU delegate using SIMD (ARM NEON / x86 AVX2). ~2-5ms inference. Default fallback when GPU/ANE unavailable. |

#### Static Library Linkage Requirements (via `build.rs`)

| Target | Rust Triple | Static Lib | Additional Link Dependencies |
| :--- | :--- | :--- | :--- |
| Android ARM64 | `aarch64-linux-android` | `libtensorflowlite_c.a` | `c++_shared` (NDK libc++) |
| Android ARM32 | `armv7-linux-androideabi` | `libtensorflowlite_c.a` | `c++_shared` (NDK libc++) |
| iOS Device | `aarch64-apple-ios` | `libtensorflowlite_c.a` | `c++`, `Accelerate.framework`, `CoreML.framework` |
| iOS Simulator | `aarch64-apple-ios-sim` | `libtensorflowlite_c.a` | `c++`, `Accelerate.framework`, `CoreML.framework` |
| Desktop (fallback) | `x86_64-*` / `aarch64-apple-darwin` | *None — uses `tract-onnx`* | N/A |

---

### 2.2 Offline / Non-Real-Time Score Following & Media Stack *(Not in Audio Loop)*

> [!NOTE]
> These dependencies run exclusively on asynchronous background workers or upon user action (e.g., importing a sheet, finishing a practice run, loading an audio file). They are **never** executed within the low-latency audio capture thread.

| Crate | Target Version | Domain Tag | Role & Non-Real-Time Capability |
| :--- | :--- | :--- | :--- |
| **`musicxml`** | `0.2` | `[OFFLINE / WORKER]` | **Sheet Music Parsing**. Parses `.musicxml` and `.mxl` files into symbolic score structures (measures, notes, rests, time signatures, dynamics). |
| **`fastdtw`** | `0.1` | `[OFFLINE / WORKER]` | **Audio-to-Score Alignment**. $O(N)$ Fast Dynamic Time Warping aligning live performance notes ([`EndNote`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs)) with reference MusicXML notes to evaluate rhythm, missed notes, and tempo stability. |
| **`symphonia`** | `0.5` | `[OFFLINE / WORKER]` | **Audio File Decoding**. Pure-Rust demuxer/decoder supporting MP3, AAC, FLAC, Vorbis, and WAV for reference tracks and backing audio. |
| **`midly`** | `0.5` | `[OFFLINE / WORKER]` | **Symbolic MIDI I/O**. Zero-allocation parser/serializer for exporting practice sessions as Standard MIDI Files (`.mid`) or reading reference MIDI files. |

---

## 3. Pure Rust Fast-Path + Spotify Basic Pitch Synergy

| Task / Feature | Handled By `MPM & SIMD DSP` | Handled By `Spotify Basic Pitch` |
| :--- | :--- | :--- |
| **Transient Attack Onset** | **Primary** (<2ms latency, sample-accurate attack spikes via Crest Factor & RealFFT Centroid) | **Secondary** (Evaluates 23ms hop onsets for polyphonic pitch assignment) |
| **Pitch Estimation** | **Monophonic Fast-Path** (Instant MPM pitch for 60 FPS visual tuner needle and telemetry) | **Polyphonic Transcription** (Multi-harmonic CNN resolving chords, overtones, and multi-notes) |
| **Note Sustain / Duration** | *Not suited* (Requires fragile debounce hangover state machines) | **Primary** (Continuous `frames[t, pitch]` activation matrix) |
| **Microtonal Pitch Bend** | **Continuous MPM Freq** (Instantaneous live pitch drift) | **`contours` Head** (3x semitone microtonal pitch bins) |

---

## 4. Module-by-Module Refactoring & File Blueprint

```
MeliorSonus/
├── tools/
│   └── build-litert/
│       ├── build_android_cmake.sh     # Invokes Android NDK CMake to produce libtensorflowlite_c.a
│       ├── build_ios_bazel.sh         # Invokes Bazel on macOS to produce iOS .a slices
│       └── CMakeLists.txt (if building custom LiteRT C wrapper)
└── shared/
    ├── build.rs                               # [BUILD] Target-conditional LiteRT static lib linkage + bindgen
    ├── native-libs/
    │   └── litert/
    │       ├── headers/
    │       │   ├── c_api.h                    # LiteRT C API header (bindgen input)
    │       │   ├── c_api_types.h              # LiteRT type definitions
    │       │   ├── common.h                   # LiteRT common enums & error codes
    │       │   └── wrapper.h                  # LiteRT C wrapper header
    │       ├── android-arm64/
    │       │   └── libtensorflowlite_c.a      # Static lib: aarch64-linux-android (CMake + NDK)
    │       ├── android-arm32/
    │       │   └── libtensorflowlite_c.a      # Static lib: armv7-linux-androideabi (CMake + NDK)
    │       ├── ios-arm64/
    │       │   └── libtensorflowlite_c.a      # Static lib: aarch64-apple-ios (Bazel)
    │       └── ios-arm64-sim/
    │           └── libtensorflowlite_c.a      # Static lib: aarch64-apple-ios-sim (Bazel)
    ├── src/commonMain/rust/
    │   ├── audio_processing/
    │   │   ├── cpal/
    │   │   │   ├── engine.rs                  # [REAL-TIME SAFE] Audio capture & rtrb producer
    │   │   │   └── listener.rs                # [BACKGROUND WORKER] Thread management & lifecycle
    │   │   ├── dsp.rs                         # [REAL-TIME SAFE] Core DSP traits & buffer defs
    │   │   ├── instruments/
    │   │   │   ├── instrument.rs              # [PEDAGOGICAL CORE] KEEP: Acoustic profiles & tolerances
    │   │   │   └── notes.rs                   # [PEDAGOGICAL CORE] KEEP: RecordNote, StartNote, EndNote
    │   │   ├── neural.rs                      # Module export + backend selection (#[cfg] feature gates)
    │   │   ├── neural/
    │   │   │   ├── litert_model.rs            # [BACKGROUND DSP WORKER] LiteRT C FFI transcriber (PRIMARY)
    │   │   │   │                              #   - Includes bindgen-generated bindings
    │   │   │   │                              #   - Implements NeuralTranscriber trait
    │   │   │   │                              #   - Configures GPU/CoreML/XNNPACK delegates
    │   │   │   │                              #   - Loads .tflite model from embedded bytes
    │   │   │   ├── tract_model.rs             # [BACKGROUND DSP WORKER] tract-onnx plan (CPU FALLBACK)
    │   │   │   │                              #   - Feature-gated: #[cfg(feature = "tract-backend")]
    │   │   │   │                              #   - Desktop/dev builds without vendored C libs
    │   │   │   ├── crnn.rs                    # Embedded model payload (.tflite primary, .onnx fallback)
    │   │   │   └── note_segmenter.rs          # [BACKGROUND DSP WORKER] Onset & frame pairing logic
    │   │   ├── processing/
    │   │   │   ├── feature_extraction/
    │   │   │   │   ├── hybrid_extractor.rs    # [BACKGROUND DSP WORKER] Glues MPM + Basic Pitch + MeliorSonus features
    │   │   │   │   ├── monophonic_feature_extractor.rs # DEPRECATE / REMOVE
    │   │   │   │   └── polyphonic_feature_extractor.rs  # DEPRECATE / REMOVE
    │   │   │   └── functions/
    │   │   │       ├── spectral/
    │   │   │       │   ├── real_fft.rs        # [REAL-TIME SAFE] Replaced with realfft wrapper
    │   │   │       │   ├── spectral_centroid.rs # [REAL-TIME SAFE] Centroid accumulator
    │   │   │       │   └── spectral_flatness.rs # [REAL-TIME SAFE] Spectral flatness
    │   │   │       ├── time_domain/
    │   │   │       │   ├── attack_slope.rs    # [REAL-TIME SAFE] Note strike velocity calculation
    │   │   │       │   ├── crest_factor.rs    # [REAL-TIME SAFE] Transient peak-to-RMS ratio
    │   │   │       │   ├── keybed_thump.rs    # [PEDAGOGICAL CORE] Sub-band impact energy (is_mashed)
    │   │   │       │   └── rms_loudness.rs    # [REAL-TIME SAFE] Frame dBFS calculation
    │   │   │       ├── filters/
    │   │   │       │   └── band_pass_filter.rs# [REAL-TIME SAFE] Replaced with biquad wrapper
    │   │   │       ├── resampling/
    │   │   │       │   └── resampler.rs       # [BACKGROUND DSP WORKER] rubato 44.1k/48k -> 22.05k pipeline
    │   │   │       └── pitch/
    │   │   │           ├── mpm.rs             # [REAL-TIME SAFE] MPM pitch tracking wrapper
    │   │   │           └── intonation.rs      # [REAL-TIME SAFE] Cents offset calculations
    │   │   └── utils/
    │   │       └── feature_extractor_state.rs # DEPRECATE / REMOVE (Handled by Basic Pitch + MPM)
    |   ├── score_following/                   # [OFFLINE / NON-REAL-TIME MODULE]
    |   │   ├── mod.rs                         # Module export
    |   │   ├── mxl_parser.rs                  # [OFFLINE / WORKER] musicxml score loader & note extractor
        │   ├── performance_aligner.rs         # [OFFLINE / WORKER] fastdtw score-to-audio alignment engine
        │   ├── midi_exporter.rs               # [OFFLINE / WORKER] midly practice session serializer
        │   └── audio_loader.rs                # [OFFLINE / WORKER] symphonia reference track audio decoder
        ├── card_output/
        │   ├── card.rs                        # [PEDAGOGICAL CORE] Pedagogical feedback cards generator
        │   └── scoring.rs                     # [OFFLINE / WORKER] Performance score calculator
        ├── constants.rs
        └── lib.rs
```

---

## 5. Execution Plan & Step-by-Step Implementation Phases

### Phase 1: Real-Time Audio Foundation & MPM Fast-Path
* **Objective:** Establish low-latency audio capture, SIMD DSP, and MPM streaming components.
* **Tasks:**
  1. Replace `ringbuf` with `rtrb::RingBuffer` in `audio_processing/cpal/engine.rs`.
  2. Implement `audio_processing/processing/functions/pitch/mpm.rs` using `pitch-detection` for zero-allocation live tuner feedback.
  3. Implement `audio_processing/processing/functions/filters/band_pass_filter.rs` using `biquad::DirectForm2Transposed`.
  4. Implement `audio_processing/processing/functions/spectral/real_fft.rs` wrapping `realfft::RealFftPlanner`.
  5. Create `audio_processing/processing/functions/resampling/resampler.rs` wrapping `rubato::FftFixedIn<f32>` (converting 44.1k/48k to 22.05k).

### Phase 2: LiteRT C FFI Integration & Neural Model Pipeline
* **Objective:** Establish hardware-accelerated neural inference via LiteRT C API with `tract-onnx` CPU fallback.
* **Tasks:**
  1. **Model Conversion:** Export Spotify Basic Pitch ONNX model and convert to `.tflite` format via `onnx2tf` + `ai-edge-litert`. Verify INT8 quantization is preserved and output tensors (`onsets[88]`, `frames[88]`, `contours[264]`) match the ONNX reference.
  2. **Build LiteRT Static Libraries:** Build `libtensorflowlite_c.a` from source for each target:
     - *Android ARM64/ARM32:* CMake + Android NDK toolchain (`-DTFLITE_C_BUILD_SHARED_LIBS=OFF`)
     - *iOS ARM64/Simulator:* Bazel on macOS (`bazel build --config=ios_arm64 -c opt //tensorflow/lite/c:tensorflowlite_c`)
  3. **Vendor Libraries:** Commit prebuilt `.a` files and C API headers (`c_api.h`, `c_api_types.h`, `common.h`) to `shared/native-libs/litert/`.
  4. **Write `build.rs`:** Implement target-conditional static library linkage:
     - Match on `TARGET` env var to select the correct architecture-specific `.a` file.
     - Link C++ stdlib (`c++` on iOS, `c++_shared` on Android).
     - Link system frameworks on iOS (`Accelerate.framework`, `CoreML.framework`).
     - Run `bindgen` against `c_api.h` to generate `litert_bindings.rs` in `OUT_DIR`.
     - Set `BINDGEN_EXTRA_CLANG_ARGS` with correct sysroot for cross-compilation targets.
  5. **Implement `litert_model.rs`:** Build `LiteRtTranscriber` struct implementing `NeuralTranscriber`:
     - `from_bytes(&[u8])` — loads embedded `.tflite` model via `TfLiteModelCreate`.
     - `transcribe_hop()` — copies audio into input tensor, invokes interpreter, extracts 3 output heads.
     - `Drop` impl for safe cleanup (`TfLiteInterpreterDelete`, `TfLiteModelDelete`, `TfLiteInterpreterOptionsDelete`).
     - `unsafe impl Send` — interpreter is single-threaded but only used from background DSP worker.
  6. **Feature-gate `tract_model.rs`:** Wrap existing `TractBasicPitchModel` with `#[cfg(feature = "tract-backend")]` for desktop/dev fallback.
  7. **Update `neural/mod.rs`:** Runtime backend selection via `#[cfg]` feature gates — `litert-backend` (default for mobile targets) vs `tract-backend` (desktop fallback).
  8. **Add `build-dependencies`** to `Cargo.toml`: `bindgen = "0.71"`, and add feature flags: `litert-backend` (default), `tract-backend`.
  9. Build `audio_processing/neural/note_segmenter.rs` to track active polyphonic notes from `onsets` and `frames` matrices.

### Phase 2b: Hardware Delegate Optimization
* **Objective:** Enable hardware-accelerated inference via LiteRT delegates for sub-2ms mobile inference.
* **Tasks:**
  1. **XNNPACK Delegate (Both Platforms):** Configure as default optimized CPU delegate via `TfLiteXNNPackDelegateCreate` with `num_threads = 2`. Verify ~2-5ms inference on ARM64.
  2. **GPU Delegate (Both Platforms):** Add `TfLiteGpuDelegateV2Create` path for floating-point models. Benchmark against XNNPACK on representative devices.
  3. **CoreML Delegate (iOS only):** Add `TfLiteCoreMlDelegateCreate` path to route inference to Apple Neural Engine (ANE) on A12+ chips. Requires linking `CoreML.framework` (already done in `build.rs`). Target: ~0.5-2ms inference.
  4. **Delegate Fallback Chain:** Implement automatic fallback: CoreML (iOS) → GPU → XNNPACK → CPU. If a delegate fails to initialize (e.g., unsupported device), gracefully fall back to next option.
  5. **Benchmark Suite:** Create `benches/inference_benchmark.rs` comparing all backends: tract-onnx CPU, LiteRT XNNPACK, LiteRT GPU, LiteRT CoreML. Report latency per-hop on target devices.

> [!NOTE]
> **NNAPI is deprecated** as of Android 15. Do **not** implement NNAPI delegate support. Use GPU delegate or XNNPACK for Android hardware acceleration.

### Phase 3: Hybrid Feature Binding & Acoustic Profiling
* **Objective:** Fuse MPM live metrics, Basic Pitch note boundaries, and MeliorSonus pedagogical features.
* **Tasks:**
  1. Build `HybridFeatureExtractor` to pair MPM live pitch metrics and Basic Pitch note spans with sub-thump keybed energy, spectral centroid, crest factor, and cents offsets.
  2. Bind `into_end_note_with_profile` to evaluate `is_mashed`, `is_flat`, `Staccato`, `Tenuto`, `Legato`, and `Marcato`.
  3. Deprecate and remove legacy `feature_extractor_state.rs` and `monophonic_feature_extractor.rs`.

### Phase 4: Offline Score Following & Reference Alignment
* **Objective:** Implement non-real-time sheet music comparison and file importing.
* **Tasks:**
  1. Implement `score_following/mxl_parser.rs` using `musicxml` crate to extract reference note sequences from `.mxl` files.
  2. Implement `score_following/performance_aligner.rs` using `fastdtw` to compute optimal alignment between performed `EndNote` events and reference score notes.
  3. Implement `score_following/audio_loader.rs` using `symphonia` for user-imported backing tracks.
  4. Implement `score_following/midi_exporter.rs` using `midly` for exporting practice performance MIDI.

### Phase 5: Verification, Benchmarking & Zero-Allocation Audit
* **Objective:** Validate real-time latency budget and cross-platform compilation.
* **Tasks:**
  1. Run `cargo clippy --all-targets` with 0 warnings.
  2. Run end-to-end integration tests in `dsp_tests.rs`.
  3. Profile audio capture thread to verify **0 dynamic heap allocations** per audio frame.
  4. Verify LiteRT static library linkage compiles cleanly for all target triples via `dev.gobley.cargo`.
  5. Validate ONNX-to-TFLite model conversion produces numerically equivalent outputs (max absolute error < 1e-4 across all 3 output heads).

---

## 6. Performance, Latency & Resource Budgets

### 6.1 Latency Budget Breakdown

#### LiteRT Primary Path (Mobile — Hardware Delegates)
* **MPM Fast-Path Pitch:** **~1.5 – 3.0 ms** (instant live UI tuner)
* **Audio Capture (`cpal` + `rtrb`):** ~5.8 ms (256 samples @ 44.1 kHz)
* **Resampling (`rubato`):** ~1.2 ms
* **Spectrogram / CQT:** ~11.6 ms (512-sample hop @ 22.05 kHz)
* **Neural Inference (LiteRT + XNNPACK):** ~2.0 – 5.0 ms
* **Neural Inference (LiteRT + GPU delegate):** ~1.0 – 3.0 ms
* **Neural Inference (LiteRT + CoreML/ANE, iOS):** ~0.5 – 2.0 ms
* **Acoustic Profiling Accumulators:** < 0.1 ms
* **Total End-to-End (XNNPACK):** **~20.6 – 25.1 ms** *(Well within < 35 ms threshold)*
* **Total End-to-End (CoreML/ANE):** **~18.6 – 22.1 ms** *(Excellent headroom)*

#### tract-onnx Fallback Path (Desktop / Dev Builds)
* **Neural Inference (`tract-onnx` ARM NEON / AVX2):** ~4.0 – 8.0 ms
* **Total End-to-End Real-Time Latency:** **~22.7 – 26.7 ms** *(Within < 35 ms threshold)*

### 6.2 Memory & Allocation Budget
* **Static Binary Footprint:** +3.8 MB (embedded INT8 model payload) + ~3-5 MB (LiteRT C static library).
* **Runtime RAM Scratch Buffers:** ~12 MB.
* **Audio Capture Thread Allocations:** **0 bytes** (strict lock-free SPSC ring buffer).
* **LiteRT Interpreter Memory:** ~2-4 MB (allocated once at model load, reused across invocations).
