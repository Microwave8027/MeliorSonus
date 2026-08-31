use crate::audio_processing::cpal::engine::AudioEngine;
use crate::audio_processing::instruments::instrument::Instrument;
use crate::audio_processing::instruments::notes::Notes;
use crate::audio_processing::neural::litert_model::HardwareDelegate;
use crate::audio_processing::processing::feature_extraction::current_feature_extractor::hybrid_extractor::PitchDetectorMode;
use crate::tests::helpers::TestErrorCallback;
use cpal::traits::{DeviceTrait, HostTrait};
use rtrb::RingBuffer;
use std::sync::Arc;

#[test]
fn test_list_input_devices() {
    let devices_res = AudioEngine::<TestErrorCallback>::list_input_devices();
    assert!(
        devices_res.is_ok(),
        "list_input_devices should return Ok: {:?}",
        devices_res.err()
    );

    let devices = devices_res.unwrap();
    println!("Found {} input device(s):", devices.len());
    let mut default_count = 0;

    for (idx, name, is_default) in &devices {
        println!("  [{}] '{}' (default: {})", idx, name, is_default);
        assert!(!name.is_empty(), "Device name should not be empty");
        if *is_default {
            default_count += 1;
        }
    }

    // There should be at most one default input device
    assert!(
        default_count <= 1,
        "There should be at most 1 default input device, got {}",
        default_count
    );

    // Verify indices are sequential
    for (i, (idx, _name, _)) in devices.iter().enumerate() {
        assert_eq!(*idx, i, "Device index should match its enumerated position");
        if let Ok(dev) = AudioEngine::<TestErrorCallback>::find_input_device(&cpal::default_host(), &i.to_string()) {
            if let Ok(def_cfg) = dev.default_input_config() {
                println!("    Default config: sample_rate={}, channels={}, format={:?}", def_cfg.sample_rate(), def_cfg.channels(), def_cfg.sample_format());
            }
            if let Ok(configs) = dev.supported_input_configs() {
                for c in configs {
                    println!("    Supported: format={:?}, channels={}, min_sr={}, max_sr={}", c.sample_format(), c.channels(), c.min_sample_rate(), c.max_sample_rate());
                }
            }
        }
    }
}

#[test]
fn test_find_input_device() {
    let host = cpal::default_host();
    let devices_res = host.input_devices();

    if let Ok(devices) = devices_res {
        let device_list: Vec<_> = devices.collect();
        if !device_list.is_empty() {
            // 1. Find by index string
            let dev_by_idx = AudioEngine::<TestErrorCallback>::find_input_device(&host, "0");
            assert!(
                dev_by_idx.is_ok(),
                "Finding input device by index '0' should succeed"
            );

            // 2. Find by exact device name
            let first_name = AudioEngine::<TestErrorCallback>::get_device_name(&device_list[0]);
            let dev_by_name =
                AudioEngine::<TestErrorCallback>::find_input_device(&host, &first_name);
            assert!(
                dev_by_name.is_ok(),
                "Finding input device by exact name '{}' should succeed",
                first_name
            );

            // 3. Find by case-insensitive name
            let dev_by_lower = AudioEngine::<TestErrorCallback>::find_input_device(
                &host,
                &first_name.to_lowercase(),
            );
            assert!(
                dev_by_lower.is_ok(),
                "Finding input device by lowercase name should succeed"
            );
        }
    }

    // 4. Finding a non-existent device should return an error with a helpful message
    let non_existent =
        AudioEngine::<TestErrorCallback>::find_input_device(&host, "__invalid_mock_device_12345__");
    assert!(
        non_existent.is_err(),
        "Finding non-existent device should return Err"
    );
    let err_msg = non_existent.err().unwrap().to_string();
    assert!(
        err_msg.contains("not found"),
        "Error message should mention 'not found', got: {}",
        err_msg
    );
}

#[test]
fn test_audio_engine_device_selection_and_switch() {
    let (prod, _cons) = RingBuffer::<Notes>::new(32);
    let mut engine = AudioEngine::new(
        Instrument::Piano,
        Arc::new(TestErrorCallback),
        prod,
        -45.0,
        PitchDetectorMode::Basic,
        HardwareDelegate::Cpu,
    );

    assert_eq!(engine.audio_device_name, None);

    // Test with_audio_device builder pattern
    engine = engine.with_audio_device(Some("Microphone 1".to_string()));
    assert_eq!(engine.audio_device_name.as_deref(), Some("Microphone 1"));

    // Test set_audio_device setter
    engine.set_audio_device(Some("Microphone 2".to_string()));
    assert_eq!(engine.audio_device_name.as_deref(), Some("Microphone 2"));

    engine.set_audio_device(None);
    assert_eq!(engine.audio_device_name, None);
}
