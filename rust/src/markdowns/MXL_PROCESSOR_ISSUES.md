# MeliorSonus MXL Processor Issues & Checklist

This document tracks the current status, known bugs, architectural edge cases, MusicXML specification compliance issues, and implementation progress across the MeliorSonus MXL preprocessing and score parsing pipeline located in [`shared/src/commonMain/rust/audio_analysis/preprocessing_mxl`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl).

---

## 📊 Pipeline Status Overview

| Subsystem | Primary Files | Status | Test Coverage |
| :--- | :--- | :--- | :--- |
| **MXL Ingestion & I/O** | [`music_xml_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/music_xml_parser.rs), [`borrowed.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/borrowed.rs) | ✅ Functional | Tested (Asset read, validation, zero-copy deserialization) |
| **Metadata Extraction** | [`metadata.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/mxl_metadata/metadata.rs) | ✅ Functional | Tested (Title, composer, part-groups, multi-part vs staves) |
| **Zero-Copy Serialization** | `rkyv` integration, [`borrowed.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/borrowed.rs) | ✅ Functional | Tested (`test_serialzation_and_parsing` on Für Elise) |
| **Measure & Note Parsing** | [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs), [`note_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/note_handler.rs) | ⚠️ Working with Edge Cases | Tested (Chords, voices, ties, grace notes, rests) |
| **Target Instrument Filtering** | [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L75-L96) | ⚠️ Note-only filter (Direction bleed) | Tested (Piano RH/LH multi-part merge, instrument substring matching) |
| **Repeat & Playback Expansion** | [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L115-L296) | ⚠️ Open Logic Flaws | Tested on standard repeats & basic voltas; bugs on edge cases |
| **Audio Processing Interop** | [`note.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/notes/note.rs), [`pitch.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/notes/pitch.rs) | ✅ Functional | Tested (`From`/`Into` conversions to DSP [`Pitch`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L12) & [`Octave`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_processing/instruments/notes.rs#L24)) |

> [!NOTE]
> All 14 automated unit tests in `cargo test --lib score_parser` and `tests::audio_analysis` currently **pass**. However, the current test suite exercises nominal happy-path cases (e.g., standard repeats, dotted "rit.", single-part scores, comma-separated voltas). The bugs documented below represent latent edge cases, multi-part score corruptions, and MusicXML spec divergences present in the active codebase.

---

## 🔴 1. Critical Logic & Playback Expansion Bugs

### [ ] 1.1 Multi-Part Cross-Contamination of Transposition, Octave Shifts, Dynamics, and Pedal
- **Files:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L56-L68), [`state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/state.rs#L4-L25)
- **Severity:** Critical
- **Status:** Open (Verified in code)
- **Description:**
  In [`iterate_over_measures_with_target_parts`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L51-L70), a single [`ParserState`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/state.rs#L4-L25) instance is allocated and passed mutably into every measure:
  ```rust
  let mut state = ParserState {
      current_divisions: 1,
      current_bpm: None,
      current_dynamic: None,
      current_transpose_semitones: 0,
      current_octave_shift: 0,
      is_pedal_active: false,
  };
  let notes: Vec<ScoreMeasure> = measures
      .iter()
      .map(|measure| parse_measure_with_target_parts(measure, &mut state, target_part_indices))
      .collect();
  ```
  Inside [`parse_measure_with_target_parts`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L191-L208), multiple parts are iterated sequentially, each mutating this exact same `state`:
  ```rust
  for (part_idx, stream) in part_streams {
      let mut current_position: u32 = 0;
      let mut last_note_start: u32 = 0;
      for elem in stream {
          process_element(elem, &mut current_position, &mut last_note_start, state, ...);
      }
  }
  ```
- **Impact:**
  - **Transposition Bleed:** If Part 1 ($B\flat$ Clarinet) specifies `<transpose><chromatic>-2</chromatic></transpose>` in Measure 1, `state.current_transpose_semitones` becomes `-2`. When Measure 2 begins, Part 0 (Flute at concert pitch) has no transposition tag, so it inherits `-2`. Flute notes sound 2 semitones flat for the remainder of the score.
  - **Octave Shift Bleed:** If Part 0 defines an `8va` shift (`<octave-shift type="down" size="8"/>`), Part 1 (e.g., Cello or Bass) inherits `current_octave_shift == 12` in the same or subsequent measures.
  - **Dynamics / Pedal Bleed:** Dynamics and sustain pedal states from one part leak across other parts in subsequent measures.
- **Recommended Fix:**
  Separate score-global state (tempo/BPM) from part-specific state. Maintain a persistent `Vec<PartParserState>` indexed by `part_idx`:
  ```rust
  pub struct ScoreParserState {
      pub current_bpm: Option<u32>,
      pub part_states: Vec<PartParserState>,
  }

  #[derive(Clone, Default)]
  pub struct PartParserState {
      pub current_divisions: u32,
      pub current_dynamic: Option<DynamicLevel>,
      pub current_transpose_semitones: i8,
      pub current_octave_shift: i8,
      pub is_pedal_active: bool,
  }
  ```

---

### [ ] 1.2 Target Part Filtering Leaks Directions, Sounds, and Attributes from Unselected Parts
- **File:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L346-L401)
- **Severity:** Critical
- **Status:** Open (Verified in code)
- **Description:**
  In [`process_element`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L346-L401), the filter check `should_include` is ONLY applied to `ScoreElement::Note`:
  ```rust
  ScoreElement::Note(note) => {
      let should_include = match target_part_indices {
          Some(indices) => indices.contains(&part_idx),
          None => true,
      };
      if should_include {
          handle_note(...);
      } else {
          advance_note_position(note, current_position, last_note_start);
      }
  }
  ```
  `ScoreElement::Attributes`, `ScoreElement::Direction`, and `ScoreElement::Sound` are executed unconditionally for every part stream regardless of `target_part_indices`.
- **Impact:**
  When filtering for a specific instrument (e.g., Piano in an orchestral or chamber score with `from_piano` or `from_instrument`), all directions, dynamics, wedge hairpins, tempo directives, octave shifts, and clefs from other instruments (Violin, Trombone, Flute) are inserted into the Piano's `inline_attrs` and mutate the shared parser state.
- **Recommended Fix:**
  Only process part-specific elements if `should_include` is true:
  ```rust
  let should_include = match target_part_indices {
      Some(indices) => indices.contains(&part_idx),
      None => true,
  };

  match elem {
      ScoreElement::Attributes(attrs) if should_include => {
          handle_attributes(attrs, *current_position, state, inline_attrs);
      }
      ScoreElement::Direction(dir) if should_include => {
          handle_direction(dir, *current_position, state, inline_attrs, repeats);
      }
      ScoreElement::Sound(sound) if should_include => {
          handle_sound(sound, *current_position, state, inline_attrs, repeats);
      }
      ...
  ```

---

### [ ] 1.3 Forward Repeat on Right Barline Repeated Section Start Off-by-One
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L231-L240)
- **Severity:** Major
- **Status:** Open (Verified in code)
- **Description:**
  In [`expand_playback_sequence`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L231-L240):
  ```rust
  RepeatVariant::Start(_) if !handled_start => {
      handled_start = true;
      if idx != repeat_start {
          repeat_start = idx;
          current_pass = 1;
          in_ending_section = false;
      }
  }
  ```
  In MusicXML and standard sheet music notation, a forward repeat barline (`|:`) may be placed either on the **left barline of measure $N$**, OR on the **right barline of measure $N - 1$** (the measure preceding the repeated section).
- **Impact:**
  When a forward repeat is on the right barline of measure $N - 1$, `idx` is $N - 1$. Setting `repeat_start = idx` incorrectly includes measure $N - 1$ inside the repeated section. When playback jumps back upon reaching the repeat end, it plays measure $N - 1$ a second time.
- **Recommended Fix:**
  Inspect `RepeatStart::location`:
  ```rust
  RepeatVariant::Start(s) if !handled_start => {
      handled_start = true;
      let target_start = if s.location == BarlineLocation::Right {
          (idx + 1).min(total.saturating_sub(1))
      } else {
          idx
      };
      if target_start != repeat_start {
          repeat_start = target_start;
          current_pass = 1;
          in_ending_section = false;
      }
  }
  ```

---

### [ ] 1.4 `direction_handler` Misses Standard Jump Directives ("D.C. al Fine", "D.S. al Coda", "Fine.")
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L83-L106)
- **Severity:** Major
- **Status:** Open (Verified in code)
- **Description:**
  Navigation directives are checked against exact string equality:
  ```rust
  } else if lower == "d.c." || lower.contains("da capo") {
      repeats.push((division_offset, RepeatVariant::Jump(RepeatJump { kind: JumpKind::DaCapo, text: Some(text.clone()) })));
  } else if lower == "d.s." || lower.contains("dal segno") {
      repeats.push((division_offset, RepeatVariant::Jump(RepeatJump { kind: JumpKind::DalSegno, text: Some(text.clone()) })));
  } else if lower == "fine" {
      repeats.push((division_offset, RepeatVariant::Jump(RepeatJump { kind: JumpKind::Fine, text: Some(text.clone()) })));
  }
  ```
- **Impact:**
  Music scores rarely print `"D.C."` in isolation; they almost universally write `"D.C. al Fine"`, `"D.C. al Coda"`, `"D.S. al Fine"`, `"D.S. al Coda"`, or `"Fine."`. Because `"d.c. al fine" != "d.c."`, every one of these standard forms fails the check and falls through to `InlineAttributeKind::DirectionWords`. Navigation jumps are completely omitted during playback expansion.
- **Recommended Fix:**
  Use prefix and word-boundary matching:
  ```rust
  let is_dc = lower.starts_with("d.c") || lower.contains("da capo");
  let is_ds = lower.starts_with("d.s") || lower.contains("dal segno");
  let is_fine = lower == "fine" || lower.starts_with("fine.") || lower.contains("al fine");
  ```

---

### [ ] 1.5 Missing Text Coda Detection & Potential Infinite Loop in `coda_idx`
- **Files:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L107-L114), [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L143-L147)
- **Severity:** Major
- **Status:** Open (Verified in code)
- **Description:**
  1. [`handle_direction`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L107) checks for `"to coda"`, but never checks for `<words>Coda</words>`. If the target Coda section is labeled with text rather than a `<coda/>` element glyph, it becomes an unparsed `DirectionWords` and `coda_idx` remains `None`.
  2. In [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L143-L147):
     ```rust
     JumpKind::Coda => {
         if coda_idx.is_none() {
             coda_idx = Some(i);
         }
     }
     ```
     If a score places a `<coda/>` sign at the "To Coda" departure point (measure $M$) as well as at the target Coda section (measure $K$), `coda_idx` records the first one ($M$).
- **Impact:**
  When `ToCoda` triggers at measure $M$, it jumps to `coda_idx` ($M$), creating an immediate infinite loop bounded only by `max_iterations`.
- **Recommended Fix:**
  - Add text matching for `"coda"` in `handle_direction` to produce `JumpKind::Coda`.
  - Disambiguate departure Coda marks from destination Coda marks: only assign `coda_idx` to measures occurring *after* any `ToCoda` mark or explicit Coda section markers.

---

## 🟡 2. MusicXML Parsing & Musical Specification Flaws

### [ ] 2.1 Multi-Pass Volta Number Parsing Fails on Space-Separated Lists
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L323-L341)
- **Severity:** Major
- **Status:** Open (Verified in code)
- **Description:**
  [`ending_includes_pass`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L323-L341) only splits on commas:
  ```rust
  pub fn ending_includes_pass(number_str: &str, pass: u32) -> bool {
      for part in number_str.split(',') {
          let part = part.trim().trim_end_matches('.');
          ...
  ```
- **Impact:**
  MusicXML exporters frequently emit space-separated lists for multi-pass endings, such as `number="1 2"` or `number="1. 2."`. `"1 2".parse::<u32>()` fails with a parse error, causing the 2nd pass of the ending to be completely skipped.
- **Recommended Fix:**
  Split on commas and whitespace simultaneously:
  ```rust
  pub fn ending_includes_pass(number_str: &str, pass: u32) -> bool {
      for part in number_str.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()) {
          let part = part.trim().trim_end_matches('.');
          if let Some((start_s, end_s)) = part.split_once('-') {
              let start = start_s.trim().trim_end_matches('.').parse::<u32>();
              let end = end_s.trim().trim_end_matches('.').parse::<u32>();
              if let (Ok(s), Ok(e)) = (start, end) {
                  if (s..=e).contains(&pass) {
                      return true;
                  }
              }
          } else if let Ok(n) = part.parse::<u32>() {
              if n == pass {
                  return true;
              }
          }
      }
      false
  }
  ```

---

### [ ] 2.2 False Positive "Ritardando" on General Italian Words
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L35-L44)
- **Severity:** Moderate
- **Status:** Open (Verified in code)
- **Description:**
  [`handle_direction`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L35-L44) performs a substring search for `"rit"`:
  ```rust
  if lower.contains("rit") || lower.contains("rall")
  ```
- **Impact:**
  Non-tempo Italian words and valid non-ritardando tempo markers contain the substring `"rit"`:
  - `"Allegro spiritoso"` / `"con spirito"`
  - `"ritmico"` / `"Allegro ritmico"`
  - `"maritimo"` / `"scritta"`
  All of these falsely trigger a [`TempoChangeKind::Ritardando`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/attributes/tempo.rs#L6) inline event.
- **Recommended Fix:**
  Match word prefixes or explicit standard abbreviations:
  ```rust
  let is_rit = lower.starts_with("rit.")
      || lower.starts_with("rit ")
      || lower == "rit"
      || lower.contains("ritard")
      || lower.contains("riten")
      || lower.contains("rall");
  ```

---

### [ ] 2.3 `cresc` and `dim` Omitted When Lacking Dots
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L66-L82)
- **Severity:** Moderate
- **Status:** Open (Verified in code)
- **Description:**
  ```rust
  } else if lower == "cresc." || lower.contains("crescendo") {
      ...
  } else if lower == "dim." || lower.contains("diminuendo") || lower.contains("decresc") {
      ...
  ```
- **Impact:**
  Scores writing `"cresc"`, `"dim"`, `"cresc poco a poco"`, or `"dim al fine"` fail the exact equality check with `"cresc."` and `"dim."`. They are dropped into unparsed `DirectionWords` instead of creating [`WedgeType::Crescendo`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/attributes/wedge.rs#L6) or `Diminuendo`.
- **Recommended Fix:**
  ```rust
  let is_cresc = lower.starts_with("cresc") || lower.contains("crescendo");
  let is_dim = lower.starts_with("dim") || lower.starts_with("decresc") || lower.contains("diminuendo");
  ```

---

### [ ] 2.4 Conflicting Dynamics Emitted When `<direction>` Contains Both `<dynamics>` and `<sound>`
- **Files:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L26-L28), [`sound_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/sound_handler.rs#L29-L37)
- **Severity:** Moderate
- **Status:** Open (Verified in code)
- **Description:**
  Notation tools (Sibelius, MuseScore) frequently serialize both elements inside a single direction:
  ```xml
  <direction>
      <direction-type><dynamics><f/></dynamics></direction-type>
      <sound dynamics="90"/>
  </direction>
  ```
  1. [`handle_sound`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/sound_handler.rs#L29-L37) parses `dynamics="90"`. In [`dynamic_from_percentage`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/sound_handler.rs#L67), `90.0 < 95.0` maps to `MezzoForte`. It pushes `Dynamic { level: MezzoForte }` and mutates `state.current_dynamic = MezzoForte`.
  2. [`handle_direction`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L132) parses `<f/>`, maps to `Forte`, pushes `Dynamic { level: Forte }`, and mutates `state.current_dynamic = Forte`.
- **Impact:**
  Two contradictory dynamic events (`MezzoForte` and `Forte`) are emitted at the exact same division offset within the same measure.
- **Recommended Fix:**
  If `direction.content.direction_type` contains a `Dynamics` element, skip dynamic extraction in `handle_sound`. Furthermore, update `dynamic_from_percentage` so that `90%` corresponds to standard `Forte` ($85 \le pct < 105$).

---

### [ ] 2.5 Metronome BPM Calculation Disregards Beat Unit (e.g., 6/8 and Cut Time)
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L156-L172)
- **Severity:** Moderate
- **Status:** Open (Verified in code)
- **Description:**
  ```rust
  if let MetronomeContents::BeatBased(beat_based) = &metronome.content {
      if let BeatEquation::BPM(per_minute) = &beat_based.equals {
          if let Ok(bpm_val) = per_minute.content.trim().parse::<f64>() {
              let bpm_u32 = bpm_val.round() as u32;
              state.current_bpm = Some(bpm_u32);
              ...
  ```
- **Impact:**
  Only `per_minute` is evaluated; `<beat-unit>` and `<beat-unit-dot>` are ignored.
  - In $6/8$ meter with a dotted-quarter beat unit set to $80$ BPM, the standard quarter-note BPM (expected by audio analysis, pitch detection, and MIDI) is $80 \times 1.5 = 120$ BPM.
  - In cut time ($2/2$) with a half-note beat unit set to $60$ BPM, the quarter-note BPM is $60 \times 2 = 120$ BPM.
  The current implementation records $80$ or $60$, distorting tempo tracking and note duration estimation in audio analysis.
- **Recommended Fix:**
  Scale `bpm_val` according to the ratio between the specified `beat_unit` / `beat_unit_dot` and a standard quarter note.

---

### [ ] 2.6 `div.max(u32::MAX - 1)` Destroys Repeat and Jump Timestamps
- **File:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L255-L263)
- **Severity:** Moderate
- **Status:** Open (Verified in code)
- **Description:**
  ```rust
  for (div, rep) in unique_repeats {
      let (key_div, priority) = match &rep {
          RepeatVariant::Start(s) if s.location == BarlineLocation::Left => (0, 0),
          RepeatVariant::Ending(e) if e.ending_type == EndingType::Start => (0, 0),
          _ => (div.max(u32::MAX - 1), 4),
      };
      items.push(((key_div, priority), ScoreContent::Repeat(rep)));
  }
  ```
- **Impact:**
  Because `u32::MAX - 1` ($4,294,967,294$) is larger than any valid division offset, `div.max(u32::MAX - 1)` evaluates to `u32::MAX - 1` for all right barlines, ending stops, Segnos, Codas, and Fine directives.
  Since [`RepeatVariant`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/repeats/variant.rs#L8-L13) does not store an internal `division_offset`, the actual timestamp where the repeat or mid-measure jump occurred is completely destroyed.
- **Recommended Fix:**
  Sort by `(div, priority)` using the actual `div`, or add `division_offset` directly into `RepeatJump` / `RepeatEnding` / `RepeatEnd`.

---

## 🔵 3. Architectural Gaps & Pipeline Integration Issues

### [ ] 3.1 Lack of Global Cumulative Playback Timeline (Absolute Division / Second Offsets)
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L299-L320)
- **Severity:** Major
- **Status:** Open
- **Description:**
  When [`unrolled_start_notes()`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L299) or [`unrolled_strike_notes()`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L307) returns notes from expanded measures, each note still carries its **measure-local** `start_division` (starting at 0 for every measure).
- **Impact:**
  The audio processing pipeline (dynamic time warping, pitch tracking alignment, and student evaluation) cannot compare transcribed microphone notes with expected notes because there is no monotonic, score-wide chronological timestamp (e.g. cumulative division count or timestamp in seconds/milliseconds) on unrolled notes.
- **Recommended Fix:**
  Provide an unrolling method (e.g., `unrolled_playback_notes_with_timing()`) that accumulates division duration across measures and uses tempo changes to compute absolute seconds:
  ```rust
  pub struct TimedPlaybackNote {
      pub note: StartNote,
      pub absolute_start_division: u64,
      pub absolute_start_time_sec: f64,
      pub duration_sec: f64,
  }
  ```

---

### [ ] 3.2 Pickup / Anacrusis Measure Duration Alignment
- **Files:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L278-L282), [`measure/attributes.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/measure/attributes.rs#L8-L12)
- **Severity:** Moderate
- **Status:** Open
- **Description:**
  Pickup measures (incomplete opening bars) are flagged via `implicit="yes"` as `full_measure: Some(false)` in `MeasureAttributes`. However, the parser does not calculate the actual musical duration of the pickup measure relative to the time signature's nominal bar duration.
- **Impact:**
  Without an explicit duration calculation for incomplete measures, cumulative beat grids and metronome sync in the UI/audio engine desynchronize by the missing remainder of the pickup bar.
- **Recommended Fix:**
  Compute `effective_measure_duration_divisions` from note/rest spans or time signature, storing it on `MeasureAttributes`.

---

### [ ] 3.3 Unvalidated Zero-Copy Memory-Mapped Deserialization (`access_unchecked`)
- **Files:** [`music_xml_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/music_xml_parser.rs#L201-L215), [`borrowed.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/borrowed.rs#L18-L36)
- **Severity:** Moderate
- **Status:** Open
- **Description:**
  `deserialize_music_score` and `deserialize_mxl_meta_data` invoke `access_unchecked::<ArchivedSequentialMusicScore>(&mmap)`.
- **Impact:**
  If an `.rkyv` cache file on disk is truncated, partially written (e.g., app process killed or storage exhausted), or modified by an external process, `access_unchecked` causes immediate undefined behavior (memory corruption or segmentation fault) rather than returning a clean `Result::Err`.
- **Recommended Fix:**
  Use `rkyv::access::<ArchivedSequentialMusicScore, _>(&mmap)` with validation to guarantee memory safety on mobile targets.

---

## 🟢 4. Compiler Warnings & Code Hygiene

### [ ] 4.1 Deprecated Function Import Warning in `sequential_score.rs`
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L10)
- **Severity:** Low (Compiler Warning)
- **Status:** Open
- **Issue:**
  ```rust
  warning: use of deprecated function `audio_analysis::preprocessing_mxl::score_parser::parser::measure_parser::resolve_target_part_index`
    --> src\commonMain\rust\audio_analysis\preprocessing_mxl\score_parser\score\sequential_score.rs:10:52
  ```
  `resolve_target_part_index` is imported but unused in `sequential_score.rs`.
- **Fix:**
  Remove `resolve_target_part_index` from imports in `sequential_score.rs`.

---

### [ ] 4.2 Clippy Style & Idiomatic Rust Suggestions in `preprocessing_mxl`
- **Files:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs), [`variant.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/repeats/variant.rs#L17-L29), [`metadata.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/mxl_metadata/metadata.rs#L40-L77), [`music_xml_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/preprocess/music_xml_parser.rs#L77-L165)
- **Severity:** Low
- **Status:** Open
- **Issues:**
  1. `measure_parser.rs:52`: `measures: &Vec<Measure>` should be passed as slice `&[Measure]`.
  2. `variant.rs:17, 21, 25, 29`: `needless_return` in `is_start`, `is_end`, `is_ending`, `is_jump`.
  3. `metadata.rs:40, 77`: Collapsible `if` statements in title and composer credit extraction.
  4. `music_xml_parser.rs:77, 118, 157`: Collapsible nested directory existence checks.
  5. `direction_handler.rs:157-158`: Collapsible `if let` blocks in metronome parsing.
  6. `sequential_score.rs:329`: Manual range comparison `pass >= s && pass <= e` should use `(s..=e).contains(&pass)`.
- **Fix:**
  Apply standard clippy recommendations across the `preprocessing_mxl` module.

---

## 🎯 Prioritized Action Checklist

```
[ ] Phase 1: Core State Isolation & Part Filtering Safety
    [ ] 1.1 Refactor ParserState into PartParserState (Vec<PartParserState>) to eliminate cross-part bleed.
    [ ] 1.2 Restrict Attributes, Directions, and Sounds in process_element to matched target parts.
    [ ] 1.3 Remove deprecated resolve_target_part_index import in sequential_score.rs.

[ ] Phase 2: Musical Notation & Playback Expansion Accuracy
    [ ] 2.1 Update BarlineLocation::Right forward repeat offset handling ((idx + 1) vs idx).
    [ ] 2.2 Fix multi-pass volta ending parsing to split on commas and whitespace simultaneously.
    [ ] 2.3 Implement prefix-based jump directive matching ("D.C.", "D.S.", "Fine.").
    [ ] 2.4 Fix "rit" / "rall" and "cresc" / "dim" prefix matching to eliminate false positives and catch dotless forms.
    [ ] 2.5 Scale metronome BPM by beat-unit ratio (dotted quarter, half note).
    [ ] 2.6 Deduplicate conflicting <sound> vs <dynamics> tags in direction_handler.
    [ ] 2.7 Preserve true division offset in RepeatVariant instead of div.max(u32::MAX - 1).

[ ] Phase 3: Global Chronological Timeline & Audio Alignment
    [ ] 3.1 Implement unrolled_playback_notes_with_timing() computing monotonic global timestamps.
    [ ] 3.2 Add effective measure duration tracking for pickup (anacrusis) measures.
    [ ] 3.3 Replace access_unchecked with validated rkyv::access.

[ ] Phase 4: Code Hygiene & Clippy Compliance
    [ ] 4.1 Replace &Vec<Measure> with &[Measure].
    [ ] 4.2 Collapse nested if blocks and remove needless returns across preprocessing_mxl.
```

