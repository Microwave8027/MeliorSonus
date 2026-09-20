use super::metadata::MxlMetaData;
use super::part_list_type::PartListType;
use musicxml::elements::ScoreTimewise;
use musicxml::parser::parse_from_xml_str;

#[test]
fn test_multi_part_metadata() {
    let xml = r#"<score-timewise>
        <work><work-title>String Quartet</work-title></work>
        <part-list>
            <score-part id="P1"><part-name>Violin 1</part-name></score-part>
            <score-part id="P2"><part-name>Violin 2</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1"></part>
            <part id="P2"></part>
        </measure>
    </score-timewise>"#;
    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let meta = MxlMetaData::from(&score.content);
    assert_eq!(meta.title, "String Quartet");
    assert_eq!(meta.instruments, vec!["Violin 1", "Violin 2"]);
    assert_eq!(meta.part_list_type, PartListType::Parts(2));
    assert_eq!(meta.song_length_bars, 1);
}

#[test]
fn test_single_part_multi_staff_metadata() {
    let xml = r#"<score-timewise>
        <work><work-title>Piano Sonata</work-title></work>
        <part-list>
            <score-part id="P1"><part-name>Piano</part-name></score-part>
        </part-list>
        <measure number="1">
            <part id="P1">
                <attributes>
                    <staves>2</staves>
                </attributes>
            </part>
        </measure>
    </score-timewise>"#;
    let score: ScoreTimewise = parse_from_xml_str(xml).unwrap();
    let meta = MxlMetaData::from(&score.content);
    assert_eq!(meta.title, "Piano Sonata");
    assert_eq!(meta.instruments, vec!["Piano"]);
    assert_eq!(meta.part_list_type, PartListType::Staves(2));
    assert_eq!(meta.song_length_bars, 1);
}
