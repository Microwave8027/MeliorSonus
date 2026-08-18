# MeliorSonus Audio Processing & DSP Issues Checklist

This checklist tracks known bugs, edge cases, state machine quirks, and performance improvements across the Rust audio engine. Check items off as you address them.

---

## 🔴 Critical & Major Logic Bugs

- [x] **1. Silence Handling Drops Polyphonic Notes & Stalls Monophonic Extractor**
  - **File:** [`polyphonic_feature_extractor.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/polyphonic_feature_extractor.rs#L231-L245)
  - **Status:** Resolved by user (draining `poly_active_notes` and delegating silent frame to `single_note_extractor`).
  - **Details:** When loudness drops below `silence_threshold_dbfs`, all active notes in `poly_active_notes` are now drained into `note_rb`, and the silent callback is forwarded to `single_note_extractor` so its active note is finalized.

- [ ] **2. Unfinalized Notes on Monophonic $\leftrightarrow$ Polyphonic Mode Transitions**
  - **File:** [`polyphonic_feature_extractor.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/polyphonic_feature_extractor.rs#L246-L283)
  - **Issue:** 
    - When transitioning from `SingleNoteFastPath` $\rightarrow$ `PolyphonicCrnnPath`, any active note in `single_note_extractor` is left hanging and never finalized.
    - When transitioning from `PolyphonicCrnnPath` / `PolyphonicHangover` $\rightarrow$ `SingleNoteFastPath`, any active notes remaining in `poly_active_notes` are left unfinalized.
  - **Fix:** Add a flush/finalize helper:
    - On entering polyphonic mode: finalize and push `single_note_extractor`'s active note.
    - On entering monophonic mode: drain and push any remaining `poly_active_notes`.

- [ ] **3. Harmonic Sieve Ghost Cascade Masking**
  - **File:** [`polyphonic_feature_extractor.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/polyphonic_feature_extractor.rs#L71-L91)
  - **Issue:** In `apply_sieve`, if low fundamental $f_0$ (e.g. C2) marks an octave harmonic ghost at $2f_0$ (e.g. C3) as `ghost_mask[C3] = true`, the loop continues. When $i$ reaches C3, `raw_probs[C3] >= 0.40` is still true because `!self.ghost_mask[i]` is not checked. As a result, the ghost at C3 acts as a candidate fundamental and erroneously masks higher legitimate notes (e.g. C4, G4).
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
  - **File:** [`cpal.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L284-L335)
  - **Issue:** In the CPAL input callback streams (`F32`, `I32`, `I16`), the channel matcher is:
    ```rust
    match channels {
        1 => { ... }
        2 => { ... }
        _ => {} // Drops 100% of audio samples!
    }
    ```
    If `build_stream` falls back to `default_input_config()` on a device with a 3+ channel mic array (common on some Android phones or USB audio interfaces), audio samples are silently discarded.
  - **Fix:** Handle arbitrary channel count $N$ by taking channel 0 or averaging:
    ```rust
    _ => {
        let ch = channels as usize;
        let _ = prod.push_iter(data.chunks_exact(ch).map(|c| c[0]));
    }
    ```

- [ ] **5. `AudioEngine::play()` Consumes `feature_extractor` Before Stream Build**
  - **File:** [`cpal.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/cpal/cpal.rs#L76-L83)
  - **Issue:** `self.feature_extractor.take().expect(...)` runs *before* `Self::build_stream(&tx)?`. If `build_stream` fails (e.g. mic permission denied, device busy), `play()` returns `Err`, but `self.feature_extractor` is now `None`. Calling `play()` again immediately panics.
  - **Fix:** Only call `self.feature_extractor.take()` after `build_stream` succeeds, or restore it on error:
    ```rust
    let (initial_stream, initial_config, consumer) = Self::build_stream(&tx)?;
    let mut feature_extractor = self.feature_extractor.take().expect("Feature extractor should be present");
    ```

---

## 🟡 State Machine & DSP Improvements

- [ ] **6. Re-articulation / Restrike of Same Pitch in `Decay` State**
  - **File:** [`feature_extractor.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/feature_extractor.rs#L150-L161)
  - **Issue:** When a note is in `Decay` state and the musician strikes the same note again (without intervening silence), `active.pitch == pitch && active.octave == octave` matches. The state machine only updates `tonality_offset` and does not detect the new note onset, nor does it update `peak_dbfs`.
  - **Fix:** Detect significant positive amplitude jumps during `Decay`:
    ```rust
    (FeatureExtractorState::Decay, Some((pitch, octave, tonality_offset))) => {
        if let Some(ref mut active) = self.active_note {
            if active.pitch == pitch && active.octave == octave {
                if dbfs > active.peak_dbfs + 3.0 {
                    // Re-articulation of the same pitch: finalize previous note and start new
                    self.finalize_note(timestamp);
                    self.start_note(pitch, octave, tonality_offset, dbfs);
                } else {
                    active.tonality_offset = tonality_offset;
                }
            } else {
                self.finalize_note(timestamp);
                self.start_note(pitch, octave, tonality_offset, dbfs);
            }
        } else {
            self.start_note(pitch, octave, tonality_offset, dbfs);
        }
    }
    ```

- [ ] **7. Dead State `FeatureExtractorState::Peak`**
  - **File:** [`feature_extractor.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/feature_extraction/feature_extractor.rs#L44-L46)
  - **Issue:** `FeatureExtractorState::Peak` is never transitioned into anywhere in the codebase (it transitions directly from `Rise` to `Decay`). The `(FeatureExtractorState::Peak, ...)` match branches are dead code.
  - **Fix:** Either remove `Peak` from `FeatureExtractorState` or implement a transition into `Peak` before `Decay`.

- [x] **8. Small $\tau$ Peak Detection Suppression in NSDF**
  - **File:** [`nsdf.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/nsdf.rs#L54-L105)
  - **Status:** Resolved by user (introduced `MIN_TAU = 8` in peak search).
  - **Details:** Setting `MIN_TAU = 8` avoids high-frequency noise spikes near $\tau \le 3$ from evaluating `(tau - k * peak_tau1).abs() <= 3` as `true` across all lags.

- [ ] **9. Duration Tracking: Sample-Count vs `Instant::now()`**
  - **File:** [`notes.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L19-L43)
  - **Issue:** `RecordNote` measures duration with `Instant::now().elapsed()`. While fine for real-time mic listening, this is subject to OS thread scheduling jitter (e.g. 5ms sleep loop) and cannot be used for offline processing (e.g. WAV file tests/benchmarking).
  - **Recommendation:** Track duration via frame/sample index (`frame_count * HOP_SIZE / sample_rate`), which provides deterministic, sample-accurate durations.

---

## 🟢 Code Hygiene & Performance

- [x] **10. Avoid Copying Frame Array by Value in Filter**
  - **File:** [`high_pass_filter.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/high_pass_filter.rs#L99)
  - **Status:** Resolved by user (updated `process_frames(&mut self, frames: &[f32])`).
  - **Details:** Passing frames by slice avoids copying 4096 bytes on the stack on every DSP hop.

- [ ] **11. Guard Against Empty Slice in `rms_dbfs.rs`**
  - **File:** [`rms_dbfs.rs`](file:///c:/Users/micro/AndroidStudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/processing/functions/rms_dbfs.rs#L13-L22)
  - **Issue:** If `raw_bytes` is empty, `rms /= raw_bytes.len() as f32` divides by 0.0, returning `NaN`.
  - **Fix:** Add `if raw_bytes.is_empty() { return -180.0; }`.

- [ ] **12. Remove Clippy Warnings**
  - **Files:** `high_pass_filter.rs`, `polyphonic_feature_extractor.rs`, `rms_dbfs.rs`
  - **Items:**
    - Needless `return` statements in `high_pass_filter.rs`
    - Collapsible `if let` / nested `if` statements in `polyphonic_feature_extractor.rs`
    - Needless range loops indexing arrays in `polyphonic_feature_extractor.rs`
