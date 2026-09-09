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
