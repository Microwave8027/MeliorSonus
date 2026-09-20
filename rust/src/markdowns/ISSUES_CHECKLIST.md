# MeliorSonus Audio Processing & DSP Issues Checklist

This checklist tracks known bugs, architectural edge cases, state machine flaws, DSP/filter corruption issues, performance bottlenecks, and project integration issues across the MeliorSonus codebase. Bugs are categorized and prioritized based on criticality, with the `dsp.rs` pipeline and real-time audio safety given highest priority.

---

## 🔴 Critical & Blocker Bugs
*Highest priority: Build/compilation failures, process crashes/panics, memory/delay line corruption, or dropped audio notes.*

- [x] **1. `DspCallBack::new` Signature / Parameter Inconsistency Causes Build Failure**
  - **Files:** [`dsp.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/dsp.rs#L32-L40), [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L221-L230), [`engine.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/engine.rs#L114-L120), [`dsp_tests.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/tests/dsp_tests.rs#L536)
  - **Status:** Open
  - **Issue:** The `DspCallBack::new` trait signature in [`dsp.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/dsp.rs#L33) expects 6 arguments with `(metrics: Option<Arc<LiveAudioMetrics>>, device: HardwareDelegate)`. [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L227-L228) implements the trait with the parameter order inverted (`device` before `metrics`), producing `E0053`. Furthermore, [`engine.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/engine.rs#L114) and [`dsp_tests.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/tests/dsp_tests.rs#L536) invoke `T::new` with only 5 arguments (omitting `device`), producing `E0061`.
  - **Fix:** Standardize `DspCallBack::new` signature across all definitions and call sites, passing `device: HardwareDelegate` through `Dsp -> AudioEngine -> T::new`.

- [x] **2. `Dsp::new` Panics on Repeated Initialization via `OnceLock::set()`**
  - **File:** [`dsp.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/dsp.rs#L74-L79)
  - **Status:** Open
  - **Issue:** `TFLITE_MODEL_PATH.set(...)` and `ONNX_MODEL_PATH.set(...)` use `.expect("Invalid model path")`. `OnceLock::set()` returns `Err(value)` if the cell is already set. If a user resets the audio engine, switches instruments, or creates a new `Dsp` instance in the same app lifecycle, calling `Dsp::new()` triggers an unrecoverable panic that crashes the host Android/iOS app.
  - **Fix:** Use `let _ = TFLITE_MODEL_PATH.set(...);` (ignoring `Err` if already populated) or query `get_or_init`.

- [x] **3. `AudioEngine::play()` Panics on Restart Due to Consumed Ring Buffer Producer (`take()`)**
  - **File:** [`engine.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/engine.rs#L94-L97)
  - **Status:** Open
  - **Issue:** In `AudioEngine::play()`, `self.rt_rb_prod.take().expect("ring buffer producer should be present")` consumes the producer. If `play()` is called after `stop()` or `reset()`, `self.rt_rb_prod` is `None`, panicking on `.expect()`. `AudioEngine::reset()` does not restore or regenerate the note ring buffer.
  - **Fix:** Either re-instantiate `(prod, cons)` on engine restart, or store a factory/generator closure that instantiates fresh ring buffers per stream lifecycle.

- [x] **4. `Box<Result<PitchDetectionModel, _>>` Type Mismatch and Unhandled Inference Initialization Failure**
  - **File:** [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L237-L241)
  - **Status:** Open
  - **Issue:** `PitchDetector::new(...)` returns `Result<PitchDetectionModel, Box<dyn Error>>`. In `HybridFeatureExtractor::new`, this is wrapped as `Box::new(PitchDetector::new(...))` which instantiates `Box<Result<...>>` rather than `Box<dyn NeuralTranscriber>`, causing compiler error `E0277`. Furthermore, `TFLITE_MODEL_PATH.get().unwrap_or("")` produces `E0308` type mismatch (`&String` vs `&str`), and runtime model loading failures are not handled with a fallback to `PitchDetectionModel::new_stub()`.
  - **Fix:** Handle model loading gracefully with fallback:
    ```rust
    let tflite_path = TFLITE_MODEL_PATH.get().map(|s| s.as_str()).unwrap_or("");
    let onnx_path = ONNX_MODEL_PATH.get().map(|s| s.as_str()).unwrap_or("");
    let model = PitchDetector::new(tflite_path, onnx_path, device)
        .unwrap_or_else(|_| PitchDetector::new_stub());
    transcriber = Some(Box::new(model));
    ```

- [x] **5. `StreamingNoteSegmenter::process_mpm_frame` Drops `StartNote` on Restrike & Legato Transitions**
  - **File:** [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L220-L268)
  - **Status:** Resolved (Changed `process_mpm_frame` to return `ArrayVec<SegmentedNoteEvent, 2>`, emitting both `End(old)` and `Start(new)` on restrikes and legato transitions).
  - **Details:** `process_mpm_frame` now returns an `ArrayVec<SegmentedNoteEvent, 2>`. When a restrike occurs (via HFC onset) or a legato pitch transition occurs, the previous active note is finalized into `EndNote` (subject to `min_note_duration_sec`), and the new note's `StartNote` is pushed immediately, preventing dropped onset events.

- [x] **6. Stateful IIR Filter Phase & Delay History Corruption Across Overlapping Frames (4x State Over-Advancement)**
  - **Files:** [`engine.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/engine.rs#L214-L227), [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L278), [`band_pass_filter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/filters/band_pass_filter.rs#L59-L65), [`keybed_thump.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/time_domain/keybed_thump.rs#L50-L65)
  - **Status:** Resolved (Implemented zero-allocation sliding FIFO window filtering only the newest `HOP_SIZE` samples).
  - **Details:** `BandPassFilter::process_hop` and `KeybedThumpDetector::evaluate_hop` filter only the newly arrived `HOP_SIZE` (512) samples chronologically without discontinuity or re-filtering. `HybridFeatureExtractor` maintains a continuous `filtered_sliding_window` of size `FRAME_SIZE`, reducing filtering CPU usage by 75% and preventing IIR feedback state corruption.

---

## 🟡 Major Logic & DSP Algorithm Bugs
*High priority: Audio thread performance bottlenecks, pedagogical feature inaccuracies, and acoustic timing desynchronization.*

- [x] **7. $O(N^2)$ Direct-Time Correlation in `NsdfEvaluator::evaluate_frame` Threatens Real-Time Audio Underruns**
  - **File:** [`nsdf.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/pitch/nsdf.rs#L27-L50)
  - **Status:** Resolved (Implemented $O(N \log N)$ Wiener-Khinchin RealFFT autocorrelation with $O(1)$ prefix-sum energy denominator).
  - **Details:** `NsdfEvaluator` now evaluates linear autocorrelation via zero-padded forward/inverse RealFFT with pre-allocated scratch buffers. Cumulative frame power $m_t(\tau)$ is computed in $O(1)$ time via squared sample prefix sums, reducing per-frame complexity from $O(N^2)$ down to $O(N \log N)$ with 0 runtime heap allocations.

- [x] **8. `RecordNote::rise_duration` Never Recorded in `StreamingNoteSegmenter`, Corrupting `attack_slope` and `Marcato` Metrics**
  - **Files:** [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L201-L280), [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L354-L358), [`articulation_classifier.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/articulation/articulation_classifier.rs#L5-L23)
  - **Status:** Resolved (Replaced synthetic rise duration with nominal ~5ms hammer contact window and updated Marcato classification using velocity and crest factor).
  - **Details:** Since piano is an impulsive struck instrument with sub-hop hammer contact time and exponential decay, `rise_duration` defaults to `0.005` (5ms) without artificial percentage approximations. `Marcato` articulation is classified via high strike velocity ($\ge 90$) and transient crest factor ($\ge 15\text{ dB}$), eliminating the polyphonic scalar envelope mapping problem.

- [x] **9. HFC Onset Restrike Timing Desynchronization (3-Frame Delayed Peak vs Current Frame Timestamp)**
  - **Files:** [`hfc_onset.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/spectral/hfc_onset.rs#L113-L165), [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L287-L290), [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L201-L248)
  - **Status:** Resolved (Passed confirmed candidate onset timestamp from `HfcOnsetDetector` to `StreamingNoteSegmenter`).
  - **Details:** `StreamingNoteSegmenter::process_mpm_frame` now takes `hfc_onset_ts: Option<u128>`. When an HFC transient is confirmed, both the `EndNote` for the preceding note and the `StartNote` / `note_striked` for the restruck note use the true peak timestamp `candidate_ts` from the onset detector, removing the ~35ms latency skew.

- [x] **10. `StreamingNoteSegmenter::process_mpm_frame` Missing Minimum Note Duration Check on Early EndNotes**
  - **File:** [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L226-L285)
  - **Status:** Resolved (Enforced `min_note_duration_sec` check before emitting `EndNote` in `process_mpm_frame`).
  - **Details:** All monophonic `EndNote` emissions in Cases 2 (restrike), 3 (legato transition), and 4 (clarity drop release) now verify `end_note.note_duration >= self.min_note_duration_sec`, rejecting single-frame noise glitches.

- [x] **11. `HarmonicSieveMasker` Missing from Hybrid Extraction Pipeline & 15% Hard Margin Over-Masking**
  - **Files:** [`harmonic_sieve.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/articulation/harmonic_sieve.rs#L60-L69), [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L173-L218)
  - **Status:** Closed, Obslete for this scenario
  - **Issue:** `HarmonicSieveMasker` is unused in `HybridFeatureExtractor::process_crnn_path`, allowing raw CRNN overtone ghost activations to reach the segmenter. Furthermore, `harmonic_sieve.rs` zeroes out note activations if `raw_probs[bin] <= mask + 0.15`, suppressing real chord notes that coincide with harmonics of lower fundamental tones unless they exceed the projected overtone leakage by more than 15%.
  - **Fix:** Integrate `HarmonicSieveMasker` into `process_crnn_path` and use acoustic energy ratio thresholds rather than an arbitrary +0.15 margin.

- [x] **12. Audio Resampler Buffer Sizing & LiteRT Fixed Tensor Input Dimension Mismatch**
  - **Files:** [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L184-L195), [`constants.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/constants.rs#L11-L13), [`litert_model.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/litert_model.rs#L597-L627)
  - **Status:** Resolved (Implemented dynamic input tensor size querying and zero-allocation stack buffer dimension matching).
  - **Details:** `LiteRtBasicPitchModel::transcribe_hop` now dynamically inspects `input_tensor_byte_size`. If the loaded model requires a specific fixed input tensor length (e.g. 256 or 512 samples) and resampled output has minor rate variance (e.g. 48kHz native rate producing 236 samples), it safely formats/zero-pads into a static stack buffer to match the tensor dimensions exactly, eliminating uninitialized tensor reads and buffer over/underflow during inference.

---

## 🟢 Minor & Code Hygiene Issues
*Low priority: Real-time safety compliance, dead code elimination, naming consistency, and IEEE-754 guards.*

- [x] **13. Heap Allocations in Inference Loop (`audio_hop_22k.to_vec()`) Violate Core Real-Time DSP Rules**
  - **File:** [`tract_model.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/tract_model.rs#L78-L84)
  - **Status:** Resolved (Replaced `to_vec()` heap allocations with direct `Array2::from_shape_fn` tensor construction).
  - **Details:** In `TractBasicPitchModel::transcribe_hop`, `Array2::from_shape_fn((1, len), |(_, j)| audio_hop_22k[j])` reads directly from the resampled audio slice to construct Tract's input tensor without creating intermediate heap `Vec<f32>` allocations.

- [x] **14. Dead Code & Orphaned Legacy Structs in Workspace**
  - **Files:** [`feature_extractor_state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/utils/feature_extractor_state.rs), [`deprecated/dsp_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/deprecated/dsp_feature_extractor.rs), [`deprecated/monophonic_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/deprecated/monophonic_feature_extractor.rs), [`deprecated/polyphonic_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/deprecated/polyphonic_feature_extractor.rs)
  - **Status:** Resolved(deleted the legacy state tracker)
  - **Issue:** The codebase contains both the deprecated feature extractors and their helper modules alongside `current_feature_extractor/hybrid_extractor.rs`.
  - **Fix:** Remove unused deprecated exports from `lib.rs` and consolidate state tracking into `StreamingNoteSegmenter`.

- [x] **15. Misleading Constant Naming (`CRNN_INPUT_SIZE` = 22050)**
  - **File:** [`constants.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/constants.rs#L12)
  - **Status:** Ignored but comment added
  - **Issue:** `pub const CRNN_INPUT_SIZE: u32 = 22050;` is named `SIZE` but represents a sampling rate (`CRNN_SAMPLE_RATE_HZ`).
  - **Fix:** Rename `CRNN_INPUT_SIZE` to `CRNN_SAMPLE_RATE` or `MODEL_SAMPLE_RATE_HZ`.

- [x] **16. `calculate_midi_velocity` Potential IEEE-754 NaN Propagation**
  - **File:** [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L47-L54)
  - **Status:** Ignored since its techncially not possible for it to be called if there is no note
  - **Issue:** If `loudness_dbfs` or `crest_factor` is `NaN`, arithmetic operations propagate `NaN` into `.round()`, which in Rust saturates/casts to 0.
  - **Fix:** Guard inputs with `.is_finite()` before calculating velocity.

---

## 🔵 Cross-Platform & KMP Project Integration Issues
*Project integration: Kotlin / KMP stream EOF and cross-platform path handling.*

- [x] **17. `fetchPdf` Missing EOF Check in `SheetSearchRepository.kt`**
  - **File:** [`SheetSearchRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SheetSearchRepository.kt#L38-L44)
  - **Status:** Resolved (Added `if (bytesRead < 0) break` check in `fetchPdf`).
  - **Details:** In `fetchPdf`, the stream reading loop `while (!response.isClosedForRead)` now includes `if (bytesRead < 0) break;` identical to `fetchMXL`, properly terminating the stream consumption when EOF is reached.

- [ ] **18. Unnormalized Path Check in `SaveSheetUseCase.kt` & `SavedSheetRepositoryImpl.kt`**
  - **Files:** [`SaveSheetUseCase.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/domain/SaveSheetUseCase.kt#L17), [`SavedSheetRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SavedSheetRepository.kt#L70)
  - **Status:** Nothing to do since the backend server is yet to be refactored, Open
  - **Issue:** Cross-platform path separator mismatch (`\` vs `/`) causes duplicate checks or deletion errors on Windows hosts. `SaveSheetUseCase.kt` checks `checkExistance(mxl = result.mxl)` using the unnormalized path while saving `normalizedMxlPath`, and `SavedSheetRepositoryImpl.deleteSheet` uses unnormalized `mxl` in `baseDir / "mxl" / mxl`.
  - **Fix:** Consistently normalize all MXL and PDF file paths before querying or storing.

- [x] **19. Disconnected `LiveAudioMetrics` Pipeline in `AudioEngine` (`Dsp::live_metrics()` Always Returns 0)**
  - **Files:** [`engine.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/engine.rs#L118-L125), [`dsp.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/dsp.rs#L77-L84), [`hybrid_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/current_feature_extractor/hybrid_extractor.rs#L148-L154)
  - **Status:** Resolved(switched to lazylock)
  - **Issue:** `Dsp::new` instantiates `Arc<LiveAudioMetrics>` and exposes `live_metrics()` and `metrics()`, but `AudioEngine::new` does not store `metrics`, and `AudioEngine::play()` passes `metrics = None` when constructing `T::new(...)`. Consequently, `HybridFeatureExtractor.metrics` is always `None`, atomic telemetry stores (`rms_dbfs_bits`, `mpm_freq_bits`, `clarity_bits`, `processed_frames`) never execute, and all UI tuner / telemetry readings return zero.
  - **Fix:** Store `metrics: Option<Arc<LiveAudioMetrics>>` in `AudioEngine` (passed from `Dsp`) and forward it to `T::new(...)` inside `AudioEngine::play`.

- [x] **20. Missing `HttpClientFactory.ios.kt` Breaks iOS Target Compilation (`[NO_ACTUAL_FOR_EXPECT]`)**
  - **Files:** [`HttpClientFactory.kt` (commonMain)](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/network/HttpClientFactory.kt#L9-L10), [`HttpClientFactory.android.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/androidMain/kotlin/com/example/meliorsonus/network/HttpClientFactory.android.kt#L18-L36), [`HttpClientFactory.ios.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/iosMain/kotlin/com/example/meliorsonus/network/HttpClientFactory.ios.kt#L9-L19)
  - **Status:** Resolved (Implemented `HttpClientFactory.ios.kt` using Ktor's Darwin engine).
  - **Details:** `HttpClientFactory.ios.kt` implements the iOS `actual` declarations for `createHttpClient` (configured with `Darwin` engine and kotlinx serialization `ContentNegotiation`) and `baseUrl`, resolving the expect/actual compilation error.

- [ ] **21. Missing iOS LiteRT Prebuilt Static Libraries (`libtensorflowlite_c.a`) Causes Linker Failure**
  - **Files:** [`build.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/build.rs#L73-L89), [`shared/native_libs/litert/ios-arm64`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/native_libs/litert/ios-arm64), [`shared/native_libs/litert/ios-arm64-sim`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/native_libs/litert/ios-arm64-sim)
  - **Status:** Open
  - **Details:** `build.rs` now checks `lib_dir.join("libtensorflowlite_c.a").exists() || lib_dir.join("libtensorflowlite_c.so").exists() || lib_dir.join("tensorflowlite_c.lib").exists()` before emitting `rustc-link-lib` directives, preventing linker crashes when directory stubs are empty.

- [x] **22. Ephemeral `NSTemporaryDirectory` Used for `appFilesDir` on iOS Causes Sheet Music Data Loss**
  - **File:** [`FileSystem.ios.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/iosMain/kotlin/com/example/meliorsonus/util/FileSystem.ios.kt#L16-L20)
  - **Status:** Resolved (Pointed `appFilesDir` to persistent `NSDocumentDirectory`).
  - **Details:** `appFilesDir` now resolves the persistent Documents directory using `NSFileManager.defaultManager.URLsForDirectory(NSDocumentDirectory, NSUserDomainMask)`, preventing sheet music and score XML data loss caused by iOS purging `NSTemporaryDirectory`.

- [x] **23. `ArrayVec` Buffer Overflow Drops Note Events During Dense Polyphony in `StreamingNoteSegmenter`**
  - **Files:** [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L6-L12), [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L158-L180), [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L304-L380)
  - **Status:** Resolved(Changed to num pitch bins * 2)
  - **Issue:** `process_crnn_frame`, `transfer_poly_to_mono`, and `finalize_all` allocate `ArrayVec<SegmentedNoteEvent, 16>` and `ArrayVec<EndNote, 16>`. Across 88 pitch bins, simultaneous chord strikes, fast runs, or pedal-release finalizations with > 16 note transitions cause `try_push` to return `Err`, silently discarding note events with `let _ = events.try_push(...)`.
  - **Fix:** Increase `MAX_POLYPHONIC_NOTES` from `16` to `NUM_PITCH_BINS * 2` (176) to guarantee no note event drops during full-keyboard polyphony or rapid restrikes.

- [x] **24. Spotify Basic Pitch `contours` Pitch Bend Head Ignored in Polyphonic Segmentation**
  - **File:** [`note_segmenter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/neural/note_segmenter.rs#L313-L315)
  - **Status:** Not changed due to piano not needing them
  - **Issue:** In `StreamingNoteSegmenter::process_crnn_frame`, `let tonality_offset = 0i8;` is hardcoded. The 264-bin microtonal pitch contour predictions (`output.contours`) computed by LiteRT and Tract are completely ignored, preventing polyphonic intonation error and pitch drift tracking.
  - **Fix:** Interpolate the 3 sub-semitone contour bins around `idx * 3` to compute exact fractional tonality offset in cents.

- [ ] **25. Decompose `DefaultComponentContext` Missing State Restoration in `MainActivity.kt`**
  - **File:** [`MainActivity.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/androidApp/src/main/kotlin/com/example/meliorsonus/MainActivity.kt#L17-L19)
  - **Status:** Open
  - **Issue:** `MainActivity` creates `DefaultComponentContext(lifecycle = essentyLifecycle())` directly instead of using `defaultComponentContext()`. On screen rotation or Android configuration changes, the navigation stack, current screen, search queries, and selected tabs are destroyed and reset back to the root `Home` tab.
  - **Fix:** Use `com.arkivanov.decompose.defaultComponentContext()` which automatically hooks `SavedStateRegistryOwner` and `ViewModelStoreOwner`.

- [ ] **26. Parent Component Retaining Mutable Child Reference Breaks Decompose Lifecycle**
  - **File:** [`RootComponent.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/ui/root/RootComponent.kt#L33-L64)
  - **Status:** Open
  - **Issue:** `DefaultRootComponent` stores `private var homeComponent: HomeComponent? = null` and invokes `homeComponent?.onReturnToHome()` inside `SheetViewer`'s `onBack` lambda. When the component stack is restored from process death, `homeComponent` is null, causing `onReturnToHome()` to silently fail.
  - **Fix:** Remove the mutable `homeComponent` field; configure `HomeComponent` to handle its own tab reset or trigger updates via state/callbacks.

- [ ] **27. `SavedSheetRepositoryImpl.deleteSheet` Path Directory Mismatch Leaves Orphan Files on Disk**
  - **Files:** [`SavedSheetRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SavedSheetRepository.kt#L68-L76), [`SheetSearchRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SheetSearchRepository.kt#L66-L69)
  - **Status:** Open
  - **Issue:** `fetchMXL` saves extracted score XML files directly in `appFilesDir / "$xmlFileName"`. However, `SavedSheetRepositoryImpl.deleteSheet` attempts to delete `baseDir / "mxl" / mxl`. Because the subfolder `"mxl"` does not exist, `exists(mxlDir)` is false, and the underlying XML file is never deleted, leaking storage.
  - **Fix:** Resolve the actual XML path directly (`mxl.toPath()`) and delete it from `appFilesDir`.

- [ ] **28. `SavedSheet` Database Retrieval Drops `pdf_icon` Reference in `HomeComponent`**
  - **File:** [`HomeComponent.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/ui/home/core/HomeComponent.kt#L106)
  - **Status:** Open
  - **Issue:** In `HomeComponent.loadSavedSheets()`, `s.pdf_icon` retrieved from the database is ignored and hardcoded to `pdf = ""`. Any sheet loaded from the saved library loses its thumbnail/PDF path reference.
  - **Fix:** Map `pdf = s.pdf_icon` when converting `SavedSheet` to `SheetSearchResult`.

- [x] **29. Hardcoded Windows Host LLVM Path in `build.rs`**
  - **File:** [`build.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/build.rs#L174-L245)
  - **Status:** Resolved (Implemented dynamic `clang -print-resource-dir`, environment variable lookups, and Unix/macOS fallback paths).
  - **Details:** `find_host_clang_include()` now executes `clang -print-resource-dir`, probes `LIBCLANG_PATH`/`LLVM_HOME`/`LLVM_PATH`, and falls back to standard Unix (`/usr/lib/clang`), macOS (Homebrew/Xcode toolchains), and Windows installation directories.

- [ ] **30. Rust Clippy Warnings & Collapsible `if` Statements in `build.rs` & DSP Modules**
  - **Files:** [`build.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/build.rs#L97-L185), [`mpm.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/pitch/mpm.rs#L39), [`nsdf.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/pitch/nsdf.rs#L70), [`real_fft.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/spectral/real_fft.rs#L49), [`tempo_tracker.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/spectral/tempo_tracker.rs#L126), [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L370-L392)
  - **Status:** Open
  - **Issue:** Running `cargo clippy -- -D warnings` fails with 8 errors on collapsible `if` statements in `build.rs`, redundant variable definitions (`let velocity` shadowed in `notes.rs`), unnecessary type casting in `mpm.rs`, and unidiomatic range indexing in `real_fft.rs` and `nsdf.rs`.
  - **Fix:** Collapse nested `if` statements in `build.rs` and apply `cargo clippy --fix`.

- [ ] **31. `PdfPreview.android.kt` Decodes PDF & Allocates Bitmaps on UI Thread**
  - **File:** [`PdfPreview.android.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/androidMain/kotlin/com/example/meliorsonus/ui/home/PdfPreview.android.kt#L27-L61)
  - **Status:** Open
  - **Issue:** PDF file I/O, `PdfRenderer` page rasterization, and `Bitmap.createBitmap` (2x resolution ARGB_8888) execute directly on the UI Main thread inside `LaunchedEffect`, causing UI stuttering and unmanaged bitmap memory accumulation.
  - **Fix:** Offload PDF rendering to `withContext(Dispatchers.IO)` and scale bitmaps according to actual display constraints.

- [ ] **32. Insecure Trust-All SSL TrustManager & Hardcoded Developer IP in `HttpClientFactory.kt`**
  - **File:** [`HttpClientFactory.kt` (androidMain)](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/androidMain/kotlin/com/example/meliorsonus/network/HttpClientFactory.kt#L20-L26), [`HttpClientFactory.kt` (androidMain)](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/androidMain/kotlin/com/example/meliorsonus/network/HttpClientFactory.kt#L36)
  - **Status:** Open
  - **Issue:** `actual val baseUrl` is hardcoded to `http://192.168.68.67:8000`, and `createHttpClient` installs a custom `X509TrustManager` with empty validation methods, leaving the client vulnerable to Man-in-the-Middle attacks.
  - **Fix:** Remove dummy `trustManager` for release builds and provide a configurable backend URL mechanism (e.g. buildConfig / environment).

- [ ] **33. `MetronomeContent.kt` Lacks Audio Output & Has Non-Reactive BPM Animation**
  - **File:** [`MetronomeContent.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/ui/sheetviewer/tabs/MetronomeContent.kt#L34-L50)
  - **Status:** Open
  - **Issue:** The metronome tab only provides a visual pendulum animation without generating audio click pulses. Additionally, changing BPM while playing does not update the running `rememberInfiniteTransition` tween duration.
  - **Fix:** Add synthesized audio tick generation via audio engine or platform audio player, and key the pendulum transition to `bpm`.

- [x] **34. Duplicated & Divergent Articulation Heuristics Between `articulation_classifier.rs` and `notes.rs`**
  - **Files:** [`articulation_classifier.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/articulation/articulation_classifier.rs#L5-L45), [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L370-L390)
  - **Status:** Resolved (Consolidated articulation, damping, and keybed strike classification into `articulation_classifier.rs` and delegated from `notes.rs`).
  - **Details:** Upgraded `classify_articulation` in `articulation_classifier.rs` with accurate multi-parameter heuristics (`duration_sec`, `is_legato`, `attack_slope`, `effective_rise_sec`, `crest_factor`, `velocity`, `profile`), resolving broken Marcato false positives on quiet high-crest notes. Added `classify_damping`, and refactored `into_end_note_with_profile` in `notes.rs` to delegate articulation, mashed keybed impact, and release damping directly to `articulation_classifier.rs`.

