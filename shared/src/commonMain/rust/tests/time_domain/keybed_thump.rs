use crate::audio_processing::processing::functions::time_domain::keybed_thump::KeybedThumpDetector;
use crate::tests::helpers::make_tone_frame;

#[test]
fn test_keybed_thump_filtering_and_detection() {
    let mut thump_detector = KeybedThumpDetector::new(44100);

    // Frame with high sub-bass thump energy (40 Hz)
    let thump_frame = make_tone_frame(40.0, 0.9, 44100);
    let thump_dbfs = thump_detector.evaluate_frame(&thump_frame);

    // Frame with only high treble tone (3000 Hz)
    let treble_frame = make_tone_frame(3000.0, 0.9, 44100);
    let treble_dbfs = thump_detector.evaluate_frame(&treble_frame);

    assert!(
        thump_dbfs > -10.0,
        "Sub-bass frame should register strong thump dBFS, got {}",
        thump_dbfs
    );
    assert!(
        thump_dbfs > treble_dbfs + 5.0,
        "Sub-bass must have higher thump energy than treble, got thump={}, treble={}",
        thump_dbfs,
        treble_dbfs
    );
}
