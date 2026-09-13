# MeliorSonus MXL Processor Issues & Checklist

This document compiles the critical logic, playback expansion, and MusicXML specification bugs identified in the MeliorSonus MXL preprocessing and score parsing pipeline (`shared/src/commonMain/rust/audio_analysis/preprocessing_mxl`).

---

## 🔴 1. Critical Logic & Playback Expansion Bugs

### [ ] 1.1 Multi-Part Cross-Contamination of Transposition, Octave Shifts, Dynamics, and Pedal
- **Files:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L17-L36), [`state.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/state.rs#L4-L12)
- **Severity:** Critical
- **Status:** Open
- **Description:**
  In [`iterate_over_measures`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L17-L36), a single [`ParserState`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/state.rs#L4-L12) instance is allocated and passed by mutable reference into every measure:
  ```rust
  let mut state = ParserState {
      current_divisions: 1,
      current_bpm: None,
      current_dynamic: None,
      current_transpose_semitones: 0,
      current_octave_shift: 0,
      is_pedal_active: false,
  };
  ```
  Inside [`parse_measure`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L128-L181), multiple parts are iterated sequentially, each mutating this exact same `state`:
  ```rust
  for (part_idx, part) in parts.iter().enumerate() {
      for element in &part.content {
          match element {
              PartElement::Attributes(attrs) => handle_attributes(attrs, current_position, state, ...),
              PartElement::Direction(dir) => handle_direction(dir, current_position, state, ...),
              ...
  ```
- **Impact:**
  - **Transposition Bleed:** If Part 1 ($B\flat$ Clarinet) specifies `<transpose><chromatic>-2</chromatic></transpose>` in Measure 1, `state.current_transpose_semitones` becomes `-2`. When Measure 2 begins, Part 0 (Flute at concert pitch) has no transposition tag, so it inherits `-2`. Flute notes sound 2 semitones flat for the remainder of the score.
  - **Octave Shift Bleed:** If Part 0 defines an `8va` shift (`<octave-shift type="down" size="8"/>`), Part 1 (e.g., Cello or Bass) inherits `current_octave_shift == 12` in the same or subsequent measures.
  - **Dynamics / Pedal Bleed:** Dynamics and sustain pedal states from one part leak across other parts in subsequent measures.
- **Recommended Fix:**
  Track part-specific state separately from score-global state (tempo/BPM). Maintain a `Vec<PartParserState>` or reset part-specific state between parts while threading persistent per-part states across measure boundaries:
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

### [ ] 1.2 Forward Repeat on Right Barline Repeated Section Start Off-by-One
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L180-L187)
- **Severity:** Major
- **Status:** Open
- **Description:**
  In [`expand_playback_sequence`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L180-L187):
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

### [ ] 1.3 `direction_handler` Misses Standard Jump Directives ("D.C. al Fine", "D.S. al Coda", "Fine.")
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L83-L106)
- **Severity:** Major
- **Status:** Open
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

### [ ] 1.4 Missing Text Coda Detection & Potential Infinite Loop in `coda_idx`
- **Files:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L107-L114), [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L80-L99)
- **Severity:** Major
- **Status:** Open
- **Description:**
  1. [`handle_direction`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L107) checks for `"to coda"`, but never checks for `<words>Coda</words>`. If the target Coda section is labeled with text rather than a `<coda/>` element glyph, it becomes an unparsed `DirectionWords` and `coda_idx` remains `None`.
  2. In [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L80-L99):
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
- **File:** [`sequential_score.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L270-L288)
- **Severity:** Major
- **Status:** Open
- **Description:**
  [`ending_includes_pass`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/score/sequential_score.rs#L270-L288) only splits on commas:
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
                  if pass >= s && pass <= e {
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
- **File:** [`direction_handler.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L36)
- **Severity:** Moderate
- **Status:** Open
- **Description:**
  [`handle_direction`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/direction_handler.rs#L36) performs a substring search for `"rit"`:
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
- **Status:** Open
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
- **Status:** Open
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
- **Status:** Open
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
  The current implementation records $80$ or $60$, distorting tempo tracking and note duration estimation.
- **Recommended Fix:**
  Scale `bpm_val` according to the ratio between the specified `beat_unit` / `beat_unit_dot` and a standard quarter note.

---

### [ ] 2.6 `div.max(u32::MAX - 1)` Destroys Repeat and Jump Timestamps
- **File:** [`measure_parser.rs`](file:///C:/Users/micro/StudioProjects/MeliorSonus/shared/src/commonMain/rust/audio_analysis/preprocessing_mxl/score_parser/parser/measure_parser.rs#L229-L236)
- **Severity:** Moderate
- **Status:** Open
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
