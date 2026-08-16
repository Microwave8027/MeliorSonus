use compose_app::prelude::*;
use cpal::traits::{DeviceTrait, HostTrait};
use std::io::Write;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
struct LiveVuCallback;

impl DspCallBack for LiveVuCallback {
    fn dsp_callback(&mut self, params: CallBackParameters) {
        // Calculate RMS (volume level) and peak amplitude
        let sum_sq: f32 = params.buffer.iter().map(|&s| s * s).sum();
        let rms = (sum_sq / params.buffer.len() as f32).sqrt();
        let peak = params.buffer.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));

        // ASCII VU Meter bar (0 to 35 characters)
        let meter_len = ((rms * 120.0) as usize).min(35);
        let bar = "=".repeat(meter_len);
        let padding = " ".repeat(35 - meter_len);

        // Run MPM pitch detection
        let pitch_hz = params.mpm.mpm(params.buffer, params.cfg, params.instrument);

        if pitch_hz > 0.0 {
            print!(
                "\r[MIC] [{}{}] RMS: {:.4} | Peak: {:.4} | Pitch: {:>7.2} Hz  ",
                bar, padding, rms, peak, pitch_hz
            );
        } else {
            print!(
                "\r[MIC] [{}{}] RMS: {:.4} | Peak: {:.4} | Pitch:   ----- Hz  ",
                bar, padding, rms, peak
            );
        }
        let _ = std::io::stdout().flush();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=======================================================");
    println!("       MeliorSonus CPAL Live Audio Capture Test        ");
    println!("=======================================================\n");

    let host = cpal::default_host();
    println!("Default Audio Host: {:?}", host.id());

    let default_device = host.default_input_device();
    if let Some(ref dev) = default_device {
        if let Ok(config) = dev.default_input_config() {
            println!("Default Input Device Config: {:?}", config);
        }
    } else {
        eprintln!("No default input device found!");
        return Ok(());
    }

    println!("\nInitializing AudioEngine with Instrument::Generic...");
    let mut engine = AudioEngine::new(Instrument::Generic, LiveVuCallback);

    println!("Starting CPAL stream...");
    engine.play()?;
    println!("Stream is live! Make noise or speak into your microphone.\n");

    // Stream for 15 seconds
    thread::sleep(Duration::from_secs(15));

    println!("\n\nShutting down stream...");
    engine.end();
    println!("Engine stopped cleanly.");

    Ok(())
}
