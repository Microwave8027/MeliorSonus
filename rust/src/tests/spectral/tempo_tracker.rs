use crate::audio_processing::processing::functions::spectral::tempo_tracker::TempoTracker;

#[test]
fn test_tempo_tracker_4_4_time_signature() {
    let mut tracker = TempoTracker::new(44100.0, 512);

    // Feed 120 BPM onsets (every ~43 frames at ~86.13 fps)
    let frames_per_beat = 43;
    let mut beats_detected = 0;
    let mut final_bpm = 0.0;

    for frame in 0..250 {
        let novelty = if frame % frames_per_beat == 0 {
            2.0
        } else {
            0.0
        };
        let (is_beat, bpm, _conf) = tracker.process_novelty_sample(novelty);
        if is_beat {
            beats_detected += 1;
        }
        final_bpm = bpm;
    }

    assert!(beats_detected >= 1);
    assert!((final_bpm - 120.0).abs() < 15.0);
}
