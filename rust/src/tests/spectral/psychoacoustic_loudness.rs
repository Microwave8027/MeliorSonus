use crate::audio_processing::instruments::notes::{
    note_to_frequency, DynamicLevel, EndNote, Notes, Octave, Pitch, StartNote,
};
use crate::audio_processing::processing::functions::spectral::psychoacoustic_loudness::PsychoacousticLoudnessMeter;
use std::f32::consts::PI;

fn generate_sine_wave(
    freq_hz: f32,
    sample_rate: f32,
    num_samples: usize,
    amplitude: f32,
) -> Vec<f32> {
    (0..num_samples)
        .map(|i| amplitude * (2.0 * PI * freq_hz * (i as f32) / sample_rate).sin())
        .collect()
}

fn calculate_rms(samples: &[f32]) -> f32 {
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

fn make_test_start_note(pitch: Pitch, octave: Octave, cents: i8) -> Notes {
    Notes::Start(StartNote {
        pitch,
        octave,
        tonality_offset: cents,
        loudness_dbfs: -20.0,
        sones: None,
        phons: None,
        note_striked: 1000,
        crest_factor: 12.0,
        sub_thump_dbfs: -60.0,
        mpm_clarity: Some(0.95),
        velocity: Some(80),
        dynamic: DynamicLevel::MezzoForte,
    })
}

fn make_test_end_note(pitch: Pitch, octave: Octave, cents: i8) -> Notes {
    Notes::End(EndNote {
        pitch,
        octave,
        tonality_offset: cents,
        avg_cents_offset: cents,
        loudness_dbfs: -20.0,
        peak_sones: None,
        peak_phons: None,
        avg_sones: None,
        avg_phons: None,
        rise_duration: 0.005,
        attack_slope: 100.0,
        note_duration: 0.5,
        note_striked: 1000,
        articulation: crate::audio_processing::instruments::notes::NoteArticulation::Normal,
        is_mashed: false,
        is_flat: false,
        spectral_centroid: 800.0,
        mpm_clarity: Some(0.95),
        velocity: Some(80),
        dynamic: DynamicLevel::MezzoForte,
        damping: crate::audio_processing::instruments::notes::DampingProfile::DryDamped,
    })
}

use crate::audio_processing::instruments::notes::pitch_octave_to_midi;

#[test]
fn test_zero_signal_produces_zero_loudness() {
    let mut meter = PsychoacousticLoudnessMeter::<2048>::new(48000);
    let silence = vec![0.0f32; 2048];
    let result = meter.process_frame(&silence);
    assert_eq!(result.total_sones, 0.0);
    assert_eq!(result.total_phons, 0.0);

    let c4 = make_test_start_note(Pitch::C, Octave::O4, 0);
    let arr = meter.calculate_frequency_loudness(&silence, &[c4.frequency().unwrap()]);
    let c4_midi = pitch_octave_to_midi(Pitch::C, Octave::O4).unwrap() as usize;
    assert_eq!(arr[c4_midi].unwrap().sones, 0.0);
    assert_eq!(arr[c4_midi].unwrap().phons, 0.0);
}

#[test]
fn test_sub_bass_no_underflow() {
    let mut meter = PsychoacousticLoudnessMeter::<2048>::new(48000);
    let silence = vec![0.0f32; 2048];
    // 15.0 Hz corresponds to MIDI note ~11 (< 21 MIDI_OFFSET). Must not panic.
    let arr = meter.calculate_frequency_loudness(&silence, &[15.0, 20.0, 25.0]);
    let midi_15hz = crate::audio_processing::processing::functions::spectral::psychoacoustic_loudness::hz_to_midi(15.0) as usize;
    assert!(arr[midi_15hz].is_some());
}

#[test]
fn test_large_polyphony_melody_not_truncated() {
    let fft_size = 2048;
    let sample_rate = 48000.0;
    let mut meter = PsychoacousticLoudnessMeter::<2048>::new(sample_rate as u32);

    // 20 distinct frequencies from 100 Hz up to 3000 Hz
    let freqs: Vec<f32> = (1..=20).map(|i| 100.0 + (i as f32) * 120.0).collect();
    let mut signal = vec![0.0f32; fft_size];
    for &f in &freqs {
        let tone = generate_sine_wave(f, sample_rate, fft_size, 0.05);
        for i in 0..fft_size {
            signal[i] += tone[i];
        }
    }

    let arr = meter.calculate_frequency_loudness(&signal, &freqs);
    // Verify the highest frequency (20th note, melody) is not truncated
    let highest_midi = crate::audio_processing::processing::functions::spectral::psychoacoustic_loudness::hz_to_midi(*freqs.last().unwrap()) as usize;
    assert!(arr[highest_midi].is_some(), "20th note must not be truncated by scratchpad");
    assert!(arr[highest_midi].unwrap().sones > 0.0);
}

#[test]
fn test_polyphonic_triad_notes_enum() {
    let fft_size = 2048;
    let sample_rate = 48000.0;
    let mut meter = PsychoacousticLoudnessMeter::<2048>::new(sample_rate as u32);

    // C Major Triad: C4 (261.63 Hz), E4 (329.63 Hz), G4 (392.00 Hz)
    let chord_pitches = [261.63f32, 329.63, 392.00];
    let mut chord_signal = vec![0.0f32; fft_size];

    for &freq in &chord_pitches {
        let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.2);
        for i in 0..fft_size {
            chord_signal[i] += tone[i];
        }
    }

    // Detected notes as enums from notes.rs (mix of Start and End notes)
    let c4 = make_test_start_note(Pitch::C, Octave::O4, 0);
    let e4 = make_test_start_note(Pitch::E, Octave::O4, 0);
    let g4 = make_test_end_note(Pitch::G, Octave::O4, 0);

    let arr = meter.calculate_frequency_loudness(
        &chord_signal,
        &[c4.frequency().unwrap(), e4.frequency().unwrap(), g4.frequency().unwrap()],
    );

    for note in &[c4, e4, g4] {
        let midi = pitch_octave_to_midi(note.pitch(), note.octave()).unwrap() as usize;
        let loud = arr[midi].expect("Loudness calculated for active note");
        println!("Note Frequency {:?}: {:.3} Sones, {:.1} Phons", note.pitch(), loud.sones, loud.phons);
        assert!(loud.sones > 0.0, "Perceived loudness must be positive for active notes");
        assert!(loud.phons > 0.0);
    }
}

#[test]
fn test_jensen_inequality_voicing_discrepancy() {
    let fft_size = 2048;
    let sample_rate = 48000.0;
    let mut meter = PsychoacousticLoudnessMeter::<2048>::new(sample_rate as u32);

    // 1. Narrow Cluster Voicing (4 notes clustered inside 1 Bark band: 400-475 Hz)
    let cluster_freqs = [400.0, 425.0, 450.0, 475.0];
    let mut cluster_signal = vec![0.0f32; fft_size];
    for &freq in &cluster_freqs {
        let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
        for i in 0..fft_size {
            cluster_signal[i] += tone[i];
        }
    }

    // 2. Open Spread Voicing (4 notes across 4 distinct Bark bands: C3, E4, C6, G7)
    let spread_freqs = [130.81, 329.63, 1046.50, 3135.96];
    let mut spread_signal = vec![0.0f32; fft_size];
    for &freq in &spread_freqs {
        let tone = generate_sine_wave(freq, sample_rate, fft_size, 0.1);
        for i in 0..fft_size {
            spread_signal[i] += tone[i];
        }
    }

    // Normalize both signals to have EXACTLY identical physical RMS energy
    let rms_cluster = calculate_rms(&cluster_signal);
    let rms_spread = calculate_rms(&spread_signal);
    for s in cluster_signal.iter_mut() {
        *s /= rms_cluster;
    }
    for s in spread_signal.iter_mut() {
        *s /= rms_spread;
    }

    let res_cluster = meter.process_frame(&cluster_signal);
    let res_spread = meter.process_frame(&spread_signal);

    println!("Cluster Voicing Loudness: {:.2} Sones", res_cluster.total_sones);
    println!("Spread Voicing Loudness:  {:.2} Sones", res_spread.total_sones);

    assert!(
        res_spread.total_sones > res_cluster.total_sones * 1.5,
        "Open spread voicing must sound at least 50% louder than narrow cluster of identical RMS (Jensen's inequality)"
    );
}



#[test]
fn test_microtonal_cents_frequency() {
    let a4_standard = note_to_frequency(Pitch::A, Octave::O4, 0).unwrap();
    assert!((a4_standard - 440.0).abs() < 0.01);

    let a4_sharp = note_to_frequency(Pitch::A, Octave::O4, 50).unwrap();
    assert!(a4_sharp > 440.0);

    let a4_flat = note_to_frequency(Pitch::A, Octave::O4, -50).unwrap();
    assert!(a4_flat < 440.0);

    let start_note = make_test_start_note(Pitch::A, Octave::O4, 25);
    assert!((start_note.frequency().unwrap() - 440.0 * 2.0f32.powf(25.0 / 1200.0)).abs() < 0.01);
}
