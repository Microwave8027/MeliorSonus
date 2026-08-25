# MeliorSonus Audio Processing & DSP Issues Checklist

This checklist tracks known bugs, edge cases, state machine quirks, performance improvements, and project integration issues across the MeliorSonus codebase. Check items off as you address them.

---

## 🔴 Critical & Major Logic Bugs

- [x] **1. Silence Handling Drops Polyphonic Notes & Stalls Monophonic Extractor**
  - **File:** [`dsp_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/dsp_feature_extractor.rs#L68-L87)
  - **Status:** Resolved (draining `poly_active_notes` and delegating silent frame to `single_note_extractor`).
  - **Details:** When loudness drops below `silence_threshold_dbfs`, all active notes in `poly_active_notes` are drained into `note_rb`, and the silent callback is forwarded to `single_note_extractor` so its active note is finalized.

- [x] **2. Unfinalized / Fragmented Notes on Monophonic $\leftrightarrow$ Polyphonic Mode Transitions**
  - **File:** [`dsp_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/dsp_feature_extractor.rs#L95-L133)
  - **Status:** Resolved (bidirectional active note migration).
  - **Details:** 
    - `SingleNoteFastPath` $\rightarrow$ `PolyphonicCrnnPath`: Ongoing `RecordNote` from `single_note_extractor` is adopted into a polyphonic slot, preserving its original strike time and duration without fragmentation.
    - `PolyphonicCrnnPath` $\rightarrow$ `SingleNoteFastPath`: Dominant active note is adopted into `single_note_extractor`, while trailing notes are finalized and pushed to `note_rb`.

- [x] **3. Harmonic Sieve Ghost Cascade Masking**
  - **File:** [`harmonic_sieve_mask.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/harmonic_sieve_mask.rs#L37-L57)
  - **Status:** Resolved(fixed via checking for ghost_mask in the structure)
  - **Fix:** Skip candidate fundamentals that have already been masked as ghosts:
    ```rust
    for i in 0..PITCH_BINS {
        if self.ghost_mask[i] || raw_probs[i] < 0.40 {
            continue;
        }
        // ...
    }
    ```

- [x] **4. Multichannel Mic Array (> 2 Channels) Drops All Audio**
  - **File:** [`cpal.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L288-L343)
  - **Status:** Resolved (handled arbitrary channel count $N$ by chunking and extracting channel 0).

- [x] **5. `AudioEngine::play()` Consumes `feature_extractor` Before Stream Build**
  - **File:** [`cpal.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L73-L81)
  - **Status:** Resolved(took the feature extractor after the stream was initalized)
  - **Fix:** Only call `self.feature_extractor.take()` after `build_stream` succeeds, or restore it on error:
    ```rust
    let (tx, rx) = mpsc::channel();
    let (initial_stream, initial_config, consumer) = Self::build_stream(&tx)?;
    let mut feature_extractor = self.feature_extractor.take().expect("Feature extractor should be present");
    ```

- [x] **6. Polyphonic Multi-Note Overwrite on Simultaneous Frame Finalization**
  - **Files:** [`polyphonic_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/polyphonic_feature_extractor.rs#L65-L141) & [`dsp_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/dsp_feature_extractor.rs#L150-L160)
  - **Status:** Resolved (`ArrayVec<Note, MAX_POLYPHONY>` frame collector).
  - **Details:** `process_polyphonic_path` returns an `ArrayVec<Note, MAX_POLYPHONY>` collecting all finalized notes in the frame without dynamic heap allocation. `dsp_feature_extractor` drains all finalized notes to `note_rb`.

---

## 🟡 State Machine & DSP Improvements

- [x] **7. Re-articulation / Restrike of Same Pitch in `Decay` State**
  - **File:** [`utils/feature_extractor_state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/utils/feature_extractor_state.rs#L119-L140)
  - **Status:** Resolved(Creates a new note if there is a sudden spike).
  - **Fix:** Detect significant positive amplitude jumps during `Decay`:
    ```rust
    (NoteEnvelopeState::Decay, Some((pitch, octave, tonality_offset))) => {
        if let Some(active) = active_note {
            if active.pitch == pitch && active.octave == octave {
                if dbfs > active.peak_dbfs + 3.0 {
                    let finalized = finalized(active_note);
                    let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs);
                    on_start(&mut record);
                    *active_note = Some(record);
                    *state = NoteEnvelopeState::Rise;
                    finalized
                } else {
                    active.tonality_offset = tonality_offset;
                    on_update(active, NoteEnvelopeState::Decay);
                    None
                }
            } else {
                let finalized = finalized(active_note);
                let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs);
                on_start(&mut record);
                *active_note = Some(record);
                *state = NoteEnvelopeState::Rise;
                finalized
            }
        } else {
            let mut record = RecordNote::new(pitch, octave, tonality_offset, dbfs);
            on_start(&mut record);
            *active_note = Some(record);
            *state = NoteEnvelopeState::Rise;
            None
        }
    }
    ```

- [x] **8. Dead State `NoteEnvelopeState::Peak`**
  - **File:** [`utils/feature_extractor_state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/utils/feature_extractor_state.rs#L60-L64) & [`#L86-L118`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/utils/feature_extractor_state.rs#L86-L118)
  - **Status:** Marked as deprecated
  - **Fix:** Either remove `Peak` from `NoteEnvelopeState` or add an explicit transition into `Peak` (e.g., when reaching peak amplitude before subsequent frame decreases).

- [x] **9. Small $\tau$ Peak Detection Suppression in NSDF**
  - **File:** [`nsdf.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/nsdf.rs#L54-L105)
  - **Status:** Resolved (introduced `MIN_TAU = 8` in peak search).

- [?] **10. Duration Tracking: Sample-Count vs `Instant::now()`**
  - **File:** [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L13-L63)
  - **Issue:** `RecordNote` measures duration with `Instant::now().elapsed()`. While fine for real-time mic listening, this is subject to OS thread scheduling jitter (e.g. 5ms sleep loop) and cannot be used for deterministic offline processing (e.g. WAV file tests/benchmarking).
  - **Status:** Should be fine for now since the app is focused on real time audio tracking not file sampling. 
  - **Recommendation:** Track duration via frame/sample index (`frame_count * HOP_SIZE / sample_rate`), which provides deterministic, sample-accurate durations.

---

## 🟢 Code Hygiene & Performance

- [x] **11. Avoid Copying Frame Array by Value in Filter**
  - **File:** [`high_pass_filter.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/high_pass_filter.rs#L99)
  - **Status:** Resolved (updated `process_frames(&mut self, frames: &[f32; FRAME_SIZE])`).

- [x] **12. Guard Against Empty Slice in `rms_dbfs.rs`**
  - **File:** [`rms_dbfs.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/rms_dbfs.rs#L13-L22)
  - **Status:** Resolved
  - **Fix:** Add `if raw_bytes.is_empty() { return -180.0; }`.

- [x] **13. Remove Clippy Warnings**
  - **Files:** `high_pass_filter.rs`, `polyphonic_feature_extractor.rs`, `harmonic_sieve_mask.rs`, `rms_dbfs.rs`, `cpal.rs`, `guard.rs`, `nsdf.rs`, `card.rs`, `dsp.rs`
  - **Status:** Resolved (0 clippy warnings across workspace).
  - **Items Addressed:**
    - Needless `return` statements and useless `format!` macros in `cpal.rs` and `rms_dbfs.rs`
    - Collapsible `if let` and nested `if` statements in `polyphonic_feature_extractor.rs` and `cpal.rs`
    - Needless range loops indexing arrays in `harmonic_sieve_mask.rs`
    - Redundant `.into()` conversions on `NoteEnvelopeState` in `polyphonic_feature_extractor.rs`
    - Implemented `Default` for `DropGuard` and `NsdfEvaluator`
    - Unnecessary casts and clones on `Copy` types in `cpal.rs`
    - Direct expression return in `card.rs` and `rms_dbfs.rs`

---

## 🔵 Project Integration & Kotlin / KMP Bugs

- [ ] **14. `fetchPdf` Missing EOF Check in `SheetSearchRepository.kt`**
  - **File:** [`SheetSearchRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SheetSearchRepository.kt#L38-L44)
  - **Issue:** In `fetchPdf`, the stream reading loop `while (!response.isClosedForRead)` lacks `if (bytesRead < 0) break;`. If `readAvailable` returns `-1` (EOF) before `isClosedForRead` flips, it can trigger infinite loops or unexpected I/O exceptions.
  - **Fix:** Add `if (bytesRead < 0) break` similar to `fetchMXL`.

- [ ] **15. Unnormalized Path Check in `SaveSheetUseCase.kt` & `SavedSheetRepositoryImpl.kt`**
  - **Files:** [`SaveSheetUseCase.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/domain/SaveSheetUseCase.kt#L17), [`SavedSheetRepository.kt`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/kotlin/com/example/meliorsonus/repository/SavedSheetRepository.kt#L70)
  - **Issue:** 
    - `SaveSheetUseCase.kt` calls `savedSheetRepository.checkExistance(mxl = result.mxl)` using raw `result.mxl`, but stores `normalizedMxlPath` (`replace("\\", "/")`). On Windows, check might fail to find an existing sheet if delimiters differ.
    - `SavedSheetRepositoryImpl.kt:deleteSheet` creates `val mxlDir = baseDir / "mxl" / mxl` using unnormalized `mxl` instead of `normalizedMxl`.
  - **Fix:** Consistently normalize all MXL and PDF file paths before querying or storing.

---

## 🔴 Critical & Major Logic Bugs (Newly Identified)

- [ ] **16. Double Filtering Advances Biquad Delay Registers Twice Per Frame**
  - **Files:** [`dsp_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/dsp_feature_extractor.rs#L65) & [`monophonic_feature_extractor.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/monophonic_feature_extractor.rs#L95)
  - **Status:** Open
  - **Issue:** In `dsp_feature_extractor::dsp_callback`, `let filtered_frame = filter.process_frames(buffer)` runs and mutates the filter state (`s1`, `s2`). Then `self.single_note_extractor.processing_single_note(buffer, cfg, filter, ...)` is called on the same frame in `SingleNoteFastPath` (and silence path), which runs `let filtered_frame = filter.process_frames(buffer)` a second time on the same `buffer`. This causes the biquad filter delay lines to advance twice as fast as the actual audio stream, corrupting filter phase/frequency response across frame boundaries and doubling filtering CPU load.
  - **Fix:** Pass the already pre-filtered frame (`&filtered_frame`) and `dbfs` into `processing_single_note`, removing redundant filtering.

- [ ] **17. `AudioEngine::reset()` Drops Supervisor Thread Without Joining & Destroys Error Reporting Arc**
  - **File:** [`cpal.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L352-L358)
  - **Status:** Open
  - **Issue:** `AudioEngine::reset()` sets `self.supervisor_handle = None;` and `self.thread_error = Arc::new(Mutex::new(None));` without signaling or joining the running background supervisor thread. The detached thread continues running in the background, competing with future audio streams, and writes any subsequent errors to an orphaned `thread_error` Mutex that is no longer accessible from `self`.
  - **Fix:** Call `self.end()` inside `reset()` before re-initializing fields to ensure the background thread is joined and cleaned up.

---

## 🟡 State Machine & DSP Improvements (Newly Identified)

- [x] **18. Note Onset Timestamp Stored as 0 in `into_note()`, Conflating Release vs Attack Time**
  - **Files:** [`notes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L68-L80) & [`feature_extractor_state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/utils/feature_extractor_state.rs#L37-L44)
  - **Status:** Resolved (Passed `onset_timestamp: u128` to `RecordNote::new` and directly populated `note.note_striked` in `into_note()`).
  - **Fix:** `RecordNote::new` now takes `note_striked: u128` when the note onset begins (Idle -> Rise, legato, or restrike). `into_note()` preserves this original onset timestamp, ensuring `note_striked` accurately reflects note onset rather than finalization/release.

- [x] **19. `HarmonicSieveMasker` Over-Masks Legitimate Harmonic Chord Notes**
  - **File:** [`harmonic_sieve_mask.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/harmonic_sieve_mask.rs#L52)
  - **Status:** Resolved (Added geometric harmonic roll-off leakage ceiling and confidence threshold `raw_probs[h_idx] < 0.65`).
  - **Fix:** Harmonic candidate ghost masking now computes expected acoustic roll-off (`raw_probs[i] * (0.80).powi(k - 1)`) and ensures notes with strong independent activation ($\ge 0.65$) are preserved in chord mixtures.

- [x] **20. `AudioEngine::pause()` / `resume()` Race with Stream Error Debounce & Rebuild**
  - **File:** [`cpal.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L91-L170)
  - **Status:** Resolved (Implemented non-blocking signal draining with persistent `is_paused` supervisor state).
  - **Fix:** The supervisor thread tracks an explicit `is_paused` state flag and non-blockingly drains signals from `rx`. When paused, it sleeps without attempting stream reconstruction or starting hardware playback, and properly resumes upon receiving `EngineSignal::Playing`.

