use crate::audio_analysis::mxl_metadata::PartListType;
use crate::audio_analysis::score_parser::attributes::{InlineAttributeKind, TempoChangeKind};
use crate::audio_analysis::score_parser::notes::{NoteCluster, Pitch};
use crate::audio_analysis::score_parser::repeats::RepeatVariant;
use crate::audio_analysis::score_parser::score::SequentialMusicScore;
use musicxml::elements::ScoreTimewise;
use musicxml::parser::parse_from_xml_str;

#[test]
fn test_simple_notes_and_clusters() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Piano</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes>
                    <divisions>4</divisions>
                </attributes>
                <direction>
                    <direction-type>
                        <dynamics><f/></dynamics>
                    </direction-type>
                </direction>
                <note>
                    <pitch>
                        <step>C</step>
                        <octave>4</octave>
                    </pitch>
                    <duration>4</duration>
                </note>
                <note>
                    <chord/>
                    <pitch>
                        <step>E</step>
                        <octave>4</octave>
                    </pitch>
                    <duration>4</duration>
                </note>
                <note>
                    <chord/>
                    <pitch>
                        <step>G</step>
                        <octave>4</octave>
                    </pitch>
                    <duration>4</duration>
                </note>
                <note>
                    <pitch>
                        <step>D</step>
                        <octave>4</octave>
                    </pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));
    assert_eq!(parsed.notes.len(), 1);

    let measure = &parsed.notes[0];
    let clusters: Vec<&NoteCluster> = measure.notes().collect();
    assert_eq!(clusters.len(), 2);

    // First cluster is C-major triad chord
    assert_eq!(clusters[0].start_division, 0);
    assert!(clusters[0].is_chord());
    assert_eq!(clusters[0].start_notes.len(), 3);
    assert_eq!(clusters[0].end_notes.len(), 3);
    assert_eq!(clusters[0].start_notes[0].pitch, Pitch::C);
    assert_eq!(clusters[0].start_notes[1].pitch, Pitch::E);
    assert_eq!(clusters[0].start_notes[2].pitch, Pitch::G);
    assert_eq!(clusters[0].end_notes[0].end_division, 4);

    // Second note is D4
    assert_eq!(clusters[1].start_division, 4);
    assert!(!clusters[1].is_chord());
    assert_eq!(clusters[1].start_notes.len(), 1);
    assert_eq!(clusters[1].end_notes.len(), 1);
    assert_eq!(clusters[1].start_notes[0].pitch, Pitch::D);
    assert_eq!(clusters[1].end_notes[0].end_division, 8);
}

#[test]
fn test_backup_and_forward() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Voice</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>8</duration>
                    <voice>1</voice>
                </note>
                <backup><duration>8</duration></backup>
                <note>
                    <pitch><step>E</step><octave>4</octave></pitch>
                    <duration>4</duration>
                    <voice>2</voice>
                </note>
                <forward><duration>4</duration></forward>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));
    let measure = &parsed.notes[0];
    let clusters: Vec<&NoteCluster> = measure.notes().collect();

    assert_eq!(clusters.len(), 1);
    // Both C4 (voice 1) and E4 (voice 2) start at division 0
    assert_eq!(clusters[0].start_division, 0);
    assert_eq!(clusters[0].start_notes.len(), 2);
    assert_eq!(clusters[0].end_notes.len(), 2);

    // Voice 1 duration is 8, voice 2 duration is 4
    assert_eq!(clusters[0].end_notes[0].end_division, 8);
    assert_eq!(clusters[0].end_notes[1].end_division, 4);
}

#[test]
fn test_inline_rits_dynamics_and_repeats() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Violin</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <barline location="left">
                    <repeat direction="forward"/>
                </barline>
                <attributes><divisions>4</divisions></attributes>
                <direction>
                    <direction-type>
                        <words>rit.</words>
                    </direction-type>
                </direction>
                <note>
                    <pitch><step>A</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
                <barline location="right">
                    <repeat direction="backward" times="2"/>
                </barline>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));
    let measure = &parsed.notes[0];

    // Check repeats
    let repeats: Vec<&RepeatVariant> = measure.repeats().collect();
    assert_eq!(repeats.len(), 2);
    assert!(repeats[0].is_start());
    assert!(repeats[1].is_end());
    assert_eq!(repeats[1].as_end().unwrap().times, 2);

    // Check rit. inline attribute
    let inline: Vec<&crate::audio_analysis::score_parser::attributes::InlineMeasureAttributes> =
        measure.inline_attributes().collect();
    assert_eq!(inline.len(), 1);
    match &inline[0].kind {
        InlineAttributeKind::TempoChange { kind, text, .. } => {
            assert_eq!(*kind, TempoChangeKind::Ritardando);
            assert_eq!(text, "rit.");
        }
        _ => panic!("Expected ritardando tempo change"),
    }
}

#[test]
fn test_divisions_metronome_and_transposition() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Bb Clarinet</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes>
                    <divisions>8</divisions>
                    <transpose>
                        <diatonic>-1</diatonic>
                        <chromatic>-2</chromatic>
                    </transpose>
                </attributes>
                <direction>
                    <direction-type>
                        <metronome>
                            <beat-unit>quarter</beat-unit>
                            <per-minute>132</per-minute>
                        </metronome>
                    </direction-type>
                </direction>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>8</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));
    let measure = &parsed.notes[0];

    // 1. Verify divisions stored on measure attributes
    assert_eq!(measure.attributes.divisions, 8);

    // 2. Verify metronome BPM extracted to inline attributes
    let inline: Vec<_> = measure.inline_attributes().collect();
    let found_bpm = inline.iter().any(|a| matches!(a.kind, InlineAttributeKind::TempoChange { bpm: Some(132), .. }));
    assert!(found_bpm, "Metronome 132 BPM should be captured");

    // 3. Verify transposition: Written C4 (MIDI 60) with chromatic -2 produces concert Bb3 (MIDI 58)
    let start_notes = measure.all_start_notes();
    assert_eq!(start_notes.len(), 1);
    assert_eq!(start_notes[0].midi_note, 58);
    assert_eq!(start_notes[0].pitch, Pitch::AsBf);
    assert_eq!(start_notes[0].tonality_offset, 0, "Tonality offset should be 0 cents");
}

#[test]
fn test_tie_continuation_and_playback_unrolling() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Piano</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <barline location="left"><repeat direction="forward"/></barline>
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                    <tie type="start"/>
                </note>
                <barline location="right"><repeat direction="backward" times="2"/></barline>
            </part>
        </measure>
        <measure number="2">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                    <tie type="stop"/>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));

    // Note 1 has tie start (is NOT continuation)
    let m1_notes = parsed.notes[0].all_start_notes();
    assert!(!m1_notes[0].is_tied_continuation());

    // Note 2 has tie stop (IS continuation, so no audio strike)
    let m2_notes = parsed.notes[1].all_start_notes();
    assert!(m2_notes[0].is_tied_continuation());

    // Playback sequence unrolling should play measure 1 twice, then measure 2 (total 3 measures)
    let expanded = parsed.expand_playback_sequence();
    assert_eq!(expanded.len(), 3);
    assert_eq!(expanded[0].attributes.measure_number, "1");
    assert_eq!(expanded[1].attributes.measure_number, "1");
    assert_eq!(expanded[2].attributes.measure_number, "2");

    // unrolled_strike_notes should exclude the tied continuation in measure 2
    let strike_notes = parsed.unrolled_strike_notes();
    assert_eq!(strike_notes.len(), 2, "Only the 2 struck notes from repeated measure 1 should be included");
}

#[test]
fn test_pedal_fermata_ornament_and_octave_shift() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Piano</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <direction>
                    <direction-type><pedal type="start"/></direction-type>
                </direction>
                <direction>
                    <direction-type><octave-shift type="down" size="8"/></direction-type>
                </direction>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                    <notations>
                        <fermata type="upright"/>
                        <ornaments><trill-mark/></ornaments>
                    </notations>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));
    let measure = &parsed.notes[0];

    let start_notes = measure.all_start_notes();
    let end_notes = measure.all_end_notes();

    // 8va shift: C4 (60) shifted by +12 becomes C5 (72)
    assert_eq!(start_notes[0].midi_note, 72);
    assert_eq!(start_notes[0].pitch, Pitch::C);
    assert_eq!(start_notes[0].octave, crate::audio_analysis::score_parser::notes::Octave::O5);

    // Fermata & Ornament
    assert!(start_notes[0].has_fermata);
    assert_eq!(start_notes[0].ornament, Some(crate::audio_analysis::score_parser::notes::OrnamentKind::Trill));

    // Pedal
    assert!(end_notes[0].is_pedaled);
}

#[test]
fn test_alternative_endings_voltas_playback_expansion() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Violin</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <barline location="left"><repeat direction="forward"/></barline>
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
        <measure number="2">
            <part id="P1">
                <barline location="left"><ending number="1" type="start"/></barline>
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>D</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
                <barline location="right">
                    <ending number="1" type="stop"/>
                    <repeat direction="backward" times="2"/>
                </barline>
            </part>
        </measure>
        <measure number="3">
            <part id="P1">
                <barline location="left"><ending number="2" type="start"/></barline>
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>E</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
                <barline location="right"><ending number="2" type="stop"/></barline>
            </part>
        </measure>
        <measure number="4">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>F</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(1));

    let expanded = parsed.expand_playback_sequence();
    let measure_numbers: Vec<&str> = expanded
        .iter()
        .map(|m| m.attributes.measure_number.as_str())
        .collect();

    // Pass 1: Measure 1 -> Measure 2 (Ending 1, repeats back to Measure 1)
    // Pass 2: Measure 1 -> Measure 3 (Ending 2, skips ending 1) -> Measure 4 (continues after voltas)
    assert_eq!(measure_numbers, vec!["1", "2", "1", "3", "4"]);
}

#[test]
fn test_multi_part_repeat_and_inline_deduplication() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Part 1</part-name></score-part>
            <score-part id="P2"><part-name>Part 2</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <barline location="left"><repeat direction="forward"/></barline>
                <attributes>
                    <divisions>4</divisions>
                    <key><fifths>2</fifths></key>
                </attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
            <part id="P2">
                <barline location="left"><repeat direction="forward"/></barline>
                <attributes>
                    <divisions>4</divisions>
                    <key><fifths>2</fifths></key>
                </attributes>
                <note>
                    <pitch><step>E</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let parsed = SequentialMusicScore::from(&score.content.measure, &PartListType::Parts(2));
    let measure = &parsed.notes[0];

    // Repeats should be deduplicated to 1 RepeatVariant::Start
    let repeats: Vec<_> = measure.repeats().collect();
    assert_eq!(repeats.len(), 1);

    // Key signature (fifths: 2) should be deduplicated to 1
    let keys: Vec<_> = measure
        .inline_attributes()
        .filter(|a| matches!(a.kind, InlineAttributeKind::KeySignature { fifths: 2 }))
        .collect();
    assert_eq!(keys.len(), 1);

    // Both notes should be preserved (part 0 and part 1)
    let notes = measure.all_start_notes();
    assert_eq!(notes.len(), 2);
}

#[test]
fn test_instrument_part_filtering() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Violin I</part-name></score-part>
            <score-part id="P2"><part-name>Grand Piano</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes>
                    <divisions>4</divisions>
                </attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
                <barline location="right"><repeat direction="backward"/></barline>
            </part>
            <part id="P2">
                <attributes>
                    <divisions>4</divisions>
                </attributes>
                <note>
                    <pitch><step>G</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
                <barline location="right"><repeat direction="backward"/></barline>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let instruments = vec!["Violin I".to_string(), "Grand Piano".to_string()];
    let parts_type = PartListType::Parts(2);

    // 1. Filtering by "piano" (case-insensitive substring) should only include Grand Piano (part 1 -> G4)
    let piano_score = SequentialMusicScore::from_instrument(
        &score.content.measure,
        &parts_type,
        &instruments,
        "piano",
    );
    let piano_notes = piano_score.notes[0].all_start_notes();
    assert_eq!(piano_notes.len(), 1);
    assert_eq!(piano_notes[0].pitch, Pitch::G);
    let piano_repeats: Vec<_> = piano_score.notes[0].repeats().collect();
    assert_eq!(piano_repeats.len(), 1, "Repeats should be deduplicated to 1, but got {:?}", piano_repeats);

    // 2. Using `from_piano` convenience method should also extract only Grand Piano
    let piano_score_convenience = SequentialMusicScore::from_piano(
        &score.content.measure,
        &parts_type,
        &instruments,
    );
    let p_notes = piano_score_convenience.notes[0].all_start_notes();
    assert_eq!(p_notes.len(), 1);
    assert_eq!(p_notes[0].pitch, Pitch::G);

    // 3. Filtering by "violin" should only include Violin I (part 0 -> C4)
    let violin_score = SequentialMusicScore::from_instrument(
        &score.content.measure,
        &parts_type,
        &instruments,
        "violin",
    );
    let violin_notes = violin_score.notes[0].all_start_notes();
    assert_eq!(violin_notes.len(), 1);
    assert_eq!(violin_notes[0].pitch, Pitch::C);

    // 4. Default parsing (without filter) keeps both notes
    let all_score = SequentialMusicScore::from(&score.content.measure, &parts_type);
    let all_notes = all_score.notes[0].all_start_notes();
    assert_eq!(all_notes.len(), 2);

    // 5. Single part fallback: when only 1 instrument exists, fallback to it even if name doesn't match query
    let single_inst = vec!["Solo Flute".to_string()];
    let single_part_score = SequentialMusicScore::from_instrument(
        &score.content.measure,
        &PartListType::Parts(1),
        &single_inst,
        "piano",
    );
    let single_notes = single_part_score.notes[0].all_start_notes();
    assert_eq!(single_notes.len(), 1);
    assert_eq!(single_notes[0].pitch, Pitch::C);
}

#[test]
fn test_split_parts_piano_filtering() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Violin</part-name></score-part>
            <score-part id="P2"><part-name>Piano (Right Hand)</part-name></score-part>
            <score-part id="P3"><part-name>Piano (Left Hand)</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
            <part id="P2">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>E</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
            <part id="P3">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>G</step><octave>3</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let instruments = vec![
        "Violin".to_string(),
        "Piano (Right Hand)".to_string(),
        "Piano (Left Hand)".to_string(),
    ];
    let parts_type = PartListType::Parts(3);

    // 1. Parsing for "piano" should include BOTH P2 (RH) and P3 (LH), and exclude P1 (Violin)
    let piano_score = SequentialMusicScore::from_piano(&score.content.measure, &parts_type, &instruments);
    let piano_notes = piano_score.notes[0].all_start_notes();
    assert_eq!(piano_notes.len(), 2, "Both piano hands (RH and LH) must be included");
    assert_eq!(piano_notes[0].pitch, Pitch::E);
    assert_eq!(piano_notes[0].part_index, 1);
    assert_eq!(piano_notes[1].pitch, Pitch::G);
    assert_eq!(piano_notes[1].part_index, 2);

    // 2. Both notes are clustered at start_division = 0
    let clusters: Vec<_> = piano_score.notes[0].notes().collect();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].start_notes.len(), 2);
    assert!(clusters[0].is_chord());

    // 3. Parsing for "violin" includes only P1
    let violin_score = SequentialMusicScore::from_instrument(&score.content.measure, &parts_type, &instruments, "violin");
    let violin_notes = violin_score.notes[0].all_start_notes();
    assert_eq!(violin_notes.len(), 1);
    assert_eq!(violin_notes[0].pitch, Pitch::C);
    assert_eq!(violin_notes[0].part_index, 0);
}

#[test]
fn test_part_group_split_parts_metadata_and_filtering() {
    let xml = r#"<score-timewise>
        <part-list>
            <score-part id="P1"><part-name>Solo Flute</part-name></score-part>
            <part-group type="start" number="1">
                <group-name>Grand Piano</group-name>
            </part-group>
            <score-part id="P2"><part-name>Right Hand</part-name></score-part>
            <score-part id="P3"><part-name>Left Hand</part-name></score-part>
            <part-group type="stop" number="1"/>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>D</step><octave>5</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
            <part id="P2">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>4</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
            <part id="P3">
                <attributes><divisions>4</divisions></attributes>
                <note>
                    <pitch><step>C</step><octave>3</octave></pitch>
                    <duration>4</duration>
                </note>
            </part>
        </measure>
    </score-timewise>"#;

    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let meta = crate::audio_analysis::mxl_metadata::MxlMetaData::from(&score.content);
    assert_eq!(meta.instruments, vec![
        "Solo Flute".to_string(),
        "Grand Piano - Right Hand".to_string(),
        "Grand Piano - Left Hand".to_string(),
    ]);

    let piano_score = SequentialMusicScore::from_piano(&score.content.measure, &meta.part_list_type, &meta.instruments);
    let piano_notes = piano_score.notes[0].all_start_notes();
    assert_eq!(piano_notes.len(), 2, "Both grouped piano parts must be included");
    assert_eq!(piano_notes[0].pitch, Pitch::C);
    assert_eq!(piano_notes[0].octave, crate::audio_analysis::score_parser::notes::Octave::O4);
    assert_eq!(piano_notes[1].pitch, Pitch::C);
    assert_eq!(piano_notes[1].octave, crate::audio_analysis::score_parser::notes::Octave::O3);
}


