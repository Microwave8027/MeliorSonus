use crate::audio_processing::processing::functions::spectral::hfc_onset::{
    HfcOnsetDetector, compute_hfc,
};

#[test]
fn test_compute_hfc_energy_weighting() {
    let mut low_spec = [0.0f32; 513];
    low_spec[5] = 1.0; // Low frequency bin

    let mut high_spec = [0.0f32; 513];
    high_spec[200] = 1.0; // High frequency bin with equal magnitude

    let hfc_low = compute_hfc(&low_spec);
    let hfc_high = compute_hfc(&high_spec);

    assert!(
        hfc_high > hfc_low * 30.0,
        "HFC must weight higher frequency bins linearly"
    );
}

#[test]
fn test_hfc_onset_detector_peak_picking_and_silence_gating() {
    let mut detector = HfcOnsetDetector::new(1.5, -45.0, 45);

    let flat_spectrum = [0.05f32; 513];
    let mut transient_spectrum = [0.05f32; 513];
    for i in 100..400 {
        transient_spectrum[i] = 1.2; // Massive transient burst in upper partials
    }

    // 1. Initial frames below window count -> None
    for i in 0..3 {
        let res = detector.process_frame(&flat_spectrum, -20.0, 1000 + (i * 10) as u128);
        assert!(res.is_none());
    }

    // 2. Candidate frame (index 3) is a huge transient onset
    let _ = detector.process_frame(&transient_spectrum, -10.0, 1030);

    // 3. Post-candidate decay frames to satisfy local peak picking
    let mut detected_ts = None;
    for i in 4..10 {
        if let Some(ts) = detector.process_frame(&flat_spectrum, -20.0, 1000 + (i * 10) as u128) {
            detected_ts = Some(ts);
        }
    }

    assert_eq!(
        detected_ts,
        Some(1030),
        "HFC onset detector should pick candidate peak at ts=1030"
    );
}
