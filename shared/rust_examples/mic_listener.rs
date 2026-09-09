use compose_app::audio_processing::{
    HybridFeatureExtractor, HybridPitchDetectorMode, PitchDetectorMode,
};
use compose_app::constants::{GLOBAL_AUDIO_METRICS, NOTE_RINGBUF_CAPACITY};
use compose_app::prelude::*;
use compose_app::utils::{LiveAudioMetrics, global_settings::GlobalSettings};
use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

struct ConsoleErrorCallback;

impl ErrorCallback for ConsoleErrorCallback {
    fn on_error(&self, msg: RustError) {
        eprintln!("\n\x1b[1;31m[Stream Error]\x1b[0m: {}", msg);
    }

    fn on_complete(&self) {
        println!("\n\x1b[1;32m[Stream Complete]\x1b[0m");
    }
}

/// Format note pitch string e.g. "A4 (+1c)" or "---"
fn format_pitch(freq: f32) -> String {
    if !(20.0..=5000.0).contains(&freq) {
        return "---".to_string();
    }
    if let Some((pitch, octave, cents)) = get_note(freq) {
        let oct_str = match octave {
            Octave::OutOfRange => "?",
            Octave::O_1 => "-1",
            Octave::O0 => "0",
            Octave::O1 => "1",
            Octave::O2 => "2",
            Octave::O3 => "3",
            Octave::O4 => "4",
            Octave::O5 => "5",
            Octave::O6 => "6",
            Octave::O7 => "7",
            Octave::O8 => "8",
            Octave::O9 => "9",
            Octave::O10 => "10",
        };
        let p_str = match pitch {
            Pitch::C => "C",
            Pitch::CsDf => "C#/Db",
            Pitch::D => "D",
            Pitch::DsEf => "D#/Eb",
            Pitch::E => "E",
            Pitch::F => "F",
            Pitch::FsGf => "F#/Gb",
            Pitch::G => "G",
            Pitch::GsAf => "G#/Ab",
            Pitch::A => "A",
            Pitch::AsBf => "A#/Bb",
            Pitch::B => "B",
            Pitch::None => "None",
        };
        format!("{}{}{:+}", p_str, oct_str, cents)
    } else {
        "---".to_string()
    }
}

/// Format dBFS visual level meter: e.g. `[████████░░░░░░░░] -24.5 dBFS`
fn format_meter(dbfs: f32) -> String {
    let safe_dbfs = if dbfs.is_nan() || dbfs.is_infinite() || dbfs < -100.0 {
        -100.0
    } else {
        dbfs
    };
    let clamped = safe_dbfs.clamp(-60.0, 0.0);
    let norm = ((clamped + 60.0) / 60.0).clamp(0.0, 1.0);
    let total_bars: usize = 16;
    let filled_bars = (norm * total_bars as f32).round() as usize;
    let empty_bars = total_bars.saturating_sub(filled_bars);
    let bar_str: String = "█".repeat(filled_bars) + &"░".repeat(empty_bars);
    if safe_dbfs <= -99.0 {
        format!("[{}]   -inf dBFS", bar_str)
    } else {
        format!("[{}] {:5.1} dBFS", bar_str, safe_dbfs)
    }
}

/// Background thread for draining the lock-free ring buffer and printing real-time status
fn run_consumer_outputter(
    mut cons: Consumer<Notes>,
    metrics: &'static LiveAudioMetrics,
    running: Arc<AtomicBool>,
) {
    let mut last_ui_refresh = Instant::now();
    let refresh_interval = Duration::from_millis(60); // ~16 FPS smooth telemetry display

    while running.load(Ordering::Relaxed) {
        // 1. Drain all pending note events produced by DSP engine
        while let Ok(note) = cons.pop() {
            // Clear current telemetry line and print prominent note banner
            print!("\r\x1B[K");
            match note {
                Notes::Start(s) => {
                    println!(
                        "NOTE START: {:?} {:?} | Vel: {:3} | Dyn: {:?} | Level: {:5.1} dBFS (ts: {} ms) | Phons: {:?} | Sones: {:?}",
                        s.pitch,
                        s.octave,
                        s.velocity,
                        s.dynamic,
                        s.loudness_dbfs,
                        s.note_striked,
                        s.phons.unwrap_or(0.0),
                        s.sones.unwrap_or(0.0)
                    );
                }
                Notes::End(e) => {
                    let cents_str = if e.is_flat {
                        format!("-{:2}c (Flat)", e.avg_cents_offset.abs())
                    } else if e.avg_cents_offset > 0 {
                        format!("+{:2}c (Sharp)", e.avg_cents_offset)
                    } else {
                        format!("{:2}c (In Tune)", e.avg_cents_offset)
                    };
                    println!(
                        "NOTE END: {:?} {:?} | Dur: {:5.2}s | Artic: {:?} | Pitch: {} | Damp: {:?} | Level: {:5.1} dBFS (ts: {} ms) | Avg Phons: {:?} | Avg Sones: {:?} | Peak Phons: {:?} | Peak Sones: {:?} ",
                        e.pitch,
                        e.octave,
                        e.note_duration,
                        e.articulation,
                        cents_str,
                        e.damping,
                        e.loudness_dbfs,
                        e.note_striked,
                        e.avg_phons,
                        e.avg_sones,
                        e.peak_phons,
                        e.peak_sones,
                    );
                }
            }
            let _ = io::stdout().flush();
        }

        // 2. Render live telemetry bar
        if last_ui_refresh.elapsed() >= refresh_interval {
            last_ui_refresh = Instant::now();
            let dbfs = metrics.rms_dbfs();
            let freq = metrics.mpm_freq();
            let clarity = metrics.clarity();
            let frames = metrics.processed_frames.load(Ordering::Relaxed);

            let pitch_str = format_pitch(freq);
            let meter_str = format_meter(dbfs);

            print!(
                "\r\x1B[K  \x1b[1;36mMeliorSonus Live\x1b[0m | {} | Pitch: {:>9} ({:6.1} Hz) | Clarity: {:3.0}% | Frames: {:>6}",
                meter_str,
                pitch_str,
                freq,
                clarity * 100.0,
                frames
            );
            let _ = io::stdout().flush();
        }

        thread::sleep(Duration::from_millis(10));
    }
}

fn parse_instrument(arg: &str) -> Option<Instrument> {
    match arg.to_lowercase().replace(['-', '_', ' '], "").as_str() {
        "piano" => Some(Instrument::Piano),
        "acousticguitar" | "guitar" => Some(Instrument::AcousticGuitar),
        "electricguitar" => Some(Instrument::ElectricGuitar),
        "bass" | "electricbass" | "electricbass4" => Some(Instrument::ElectricBass4),
        "electricbass5" => Some(Instrument::ElectricBass5),
        "violin" => Some(Instrument::Violin),
        "viola" => Some(Instrument::Viola),
        "cello" => Some(Instrument::Cello),
        "doublebass" => Some(Instrument::DoubleBass),
        "flute" => Some(Instrument::Flute),
        "clarinet" | "clarinetbb" => Some(Instrument::ClarinetBb),
        "oboe" => Some(Instrument::Oboe),
        "bassoon" => Some(Instrument::Bassoon),
        "altosax" | "sax" | "saxophone" => Some(Instrument::AltoSax),
        "tenorsax" => Some(Instrument::TenorSax),
        "trumpet" | "trumpetbb" => Some(Instrument::TrumpetBb),
        "frenchhorn" | "horn" => Some(Instrument::FrenchHorn),
        "trombone" | "trombonetenor" => Some(Instrument::TromboneTenor),
        "tuba" => Some(Instrument::Tuba),
        "soprano" | "voicesoprano" => Some(Instrument::VoiceSoprano),
        "tenor" | "voicetenor" => Some(Instrument::VoiceTenor),
        "voicebass" | "bassvoice" | "voice" => Some(Instrument::VoiceBass),
        "generic" => Some(Instrument::Generic),
        _ => None,
    }
}

fn parse_mode(arg: &str) -> Option<PitchDetectorMode> {
    match arg.to_lowercase().as_str() {
        "basic" | "mpm" | "mono" | "monophonic" => Some(PitchDetectorMode::Basic),
        "hybrid" | "auto" => Some(PitchDetectorMode::Hybrid),
        "crnn" | "neural" | "basicpitch" | "poly" | "polyphonic" => Some(PitchDetectorMode::Crnn),
        _ => None,
    }
}

fn parse_hardware(arg: &str) -> Option<HardwareDelegate> {
    match arg.to_lowercase().as_str() {
        "cpu" => Some(HardwareDelegate::Cpu),
        "gpu" => Some(HardwareDelegate::Gpu),
        "npu" => Some(HardwareDelegate::Npu),
        "auto" => Some(HardwareDelegate::Auto),
        _ => None,
    }
}

struct CliConfig {
    device: Option<String>,
    instrument: Instrument,
    threshold_dbfs: f32,
    mode: PitchDetectorMode,
    hardware: HardwareDelegate,
    list_devices_only: bool,
    show_help: bool,
}

fn parse_cli_args() -> CliConfig {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut config = CliConfig {
        device: None,
        instrument: Instrument::Piano,
        threshold_dbfs: -45.0,
        mode: PitchDetectorMode::Basic,
        hardware: HardwareDelegate::Cpu,
        list_devices_only: false,
        show_help: false,
    };

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" | "help" => {
                config.show_help = true;
                return config;
            }
            "-l" | "--list" | "--list-devices" | "list" => {
                config.list_devices_only = true;
                return config;
            }
            "-d" | "--device" => {
                if i + 1 < args.len() {
                    i += 1;
                    config.device = Some(args[i].clone());
                }
            }
            "-i" | "--instrument" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(inst) = parse_instrument(&args[i]) {
                        config.instrument = inst;
                    }
                }
            }
            "-t" | "--threshold" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(t) = args[i].parse::<f32>() {
                        config.threshold_dbfs = t;
                    }
                }
            }
            "-m" | "--mode" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(m) = parse_mode(&args[i]) {
                        config.mode = m;
                    }
                }
            }
            "-g" | "--hardware" | "--delegate" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Some(hw) = parse_hardware(&args[i]) {
                        config.hardware = hw;
                    }
                }
            }
            other => {
                if other.starts_with("--device=") {
                    config.device = Some(other["--device=".len()..].to_string());
                } else if other.starts_with("-d=") {
                    config.device = Some(other["-d=".len()..].to_string());
                } else if other.starts_with("--instrument=") {
                    if let Some(inst) = parse_instrument(&other["--instrument=".len()..]) {
                        config.instrument = inst;
                    }
                } else if other.starts_with("--threshold=") {
                    if let Ok(t) = other["--threshold=".len()..].parse::<f32>() {
                        config.threshold_dbfs = t;
                    }
                } else if other.starts_with("--mode=") {
                    if let Some(m) = parse_mode(&other["--mode=".len()..]) {
                        config.mode = m;
                    }
                } else if other.starts_with("--hardware=") {
                    if let Some(hw) = parse_hardware(&other["--hardware=".len()..]) {
                        config.hardware = hw;
                    }
                } else if !other.starts_with('-') {
                    // Positional argument interpretation:
                    // 1. If it parses as instrument -> instrument
                    // 2. If it parses as mode -> mode
                    // 3. If it parses as hardware -> hardware
                    // 4. If device is not set yet -> device! (supports index "0", "1" or name "Realtek")
                    // 5. Otherwise, if it parses as float -> threshold
                    if let Some(inst) = parse_instrument(other) {
                        config.instrument = inst;
                    } else if let Some(m) = parse_mode(other) {
                        config.mode = m;
                    } else if let Some(hw) = parse_hardware(other) {
                        config.hardware = hw;
                    } else if config.device.is_none() {
                        config.device = Some(other.to_string());
                    } else if let Ok(t) = other.parse::<f32>() {
                        config.threshold_dbfs = t;
                    }
                } else if other.starts_with('-') {
                    // Negative float threshold (e.g. -40.0)
                    if let Ok(t) = other.parse::<f32>() {
                        config.threshold_dbfs = t;
                    }
                }
            }
        }
        i += 1;
    }

    config
}

fn print_help() {
    println!(
        r#"
  MeliorSonus Real-Time Microphone DSP Listener & Pedagogical Analyzer

USAGE:
    cargo run --example mic_listener -- [OPTIONS] [DEVICE] [INSTRUMENT] [THRESHOLD]

ARGUMENTS:
    [DEVICE]             Input device name (fuzzy match) or index number (e.g. 0, "Built-in", "Realtek")
    [INSTRUMENT]         Instrument preset (e.g. piano, guitar, violin, trumpet, alto_sax, voice)
    [THRESHOLD]          Silence threshold in dBFS (e.g. -45.0)

OPTIONS:
    -d, --device <NAME|IDX>    Select audio input microphone device by name or index
    -l, --list, --list-devices List all available host input audio devices and exit
    -i, --instrument <NAME>    Set target instrument acoustic model (default: piano)
    -t, --threshold <DBFS>     Set silence gating threshold in dBFS (default: -45.0)
    -m, --mode <MODE>          Pitch detector mode: basic (MPM mono), hybrid, crnn (neural poly)
    -g, --hardware <DELEGATE>  Hardware delegate for neural inference: cpu, gpu, npu, auto (default: cpu)
    -h, --help                 Display this help message

EXAMPLES:
    # List all available microphone input devices
    cargo run --example mic_listener -- --list

    # Run with default microphone
    cargo run --example mic_listener

    # Run on device index 0 for trumpet with -40 dBFS threshold
    cargo run --example mic_listener -- --device 0 --instrument trumpet --threshold -40.0

    # Fuzzy-match device by name with hybrid pitch detection
    cargo run --example mic_listener -- -d "USB Audio" -i guitar -m hybrid
"#
    );
}

fn print_available_devices() -> Result<(), Box<dyn std::error::Error>> {
    let devices = AudioEngine::<ConsoleErrorCallback>::list_input_devices()?;
    println!("\nAvailable Audio Input Devices:");
    if devices.is_empty() {
        println!("  (No input devices found on host!)");
    } else {
        for (idx, name, is_default) in devices {
            let default_tag = if is_default {
                " \x1b[1;33m[DEFAULT]\x1b[0m"
            } else {
                ""
            };
            println!("  \x1b[1;36m[{}]\x1b[0m {}{}", idx, name, default_tag);
        }
    }
    println!();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_flush_to_zero();
    let config = parse_cli_args();

    if config.show_help {
        print_help();
        return Ok(());
    }

    if config.list_devices_only {
        print_available_devices()?;
        return Ok(());
    }

    println!("============================================================");
    println!("  MeliorSonus Real-Time Microphone DSP Listener & Pedagogical Analyzer");
    println!("============================================================");

    // List available input devices and resolve chosen device
    let available_devices = AudioEngine::<ConsoleErrorCallback>::list_input_devices()?;
    println!("\nDetected Host Audio Input Devices:");
    if available_devices.is_empty() {
        eprintln!("  \x1b[1;31mWarning: No input devices found on default audio host!\x1b[0m");
    } else {
        for (idx, name, is_default) in &available_devices {
            let default_tag = if *is_default {
                " \x1b[1;33m[DEFAULT]\x1b[0m"
            } else {
                ""
            };
            let is_matched = match &config.device {
                Some(target) => {
                    if let Ok(target_idx) = target.parse::<usize>() {
                        target_idx == *idx
                    } else {
                        name.to_lowercase().contains(&target.to_lowercase())
                    }
                }
                None => *is_default,
            };
            let selected_tag = if is_matched {
                " \x1b[1;32m◀ SELECTED\x1b[0m"
            } else {
                ""
            };
            println!(
                "  \x1b[1;36m[{}]\x1b[0m {}{}{}",
                idx, name, default_tag, selected_tag
            );
        }
    }

    println!("\nConfiguration:");
    println!(
        "  • Device Target:     {}",
        config
            .device
            .as_deref()
            .unwrap_or("System Default Microphone")
    );
    println!("  • Instrument Preset: {:?}", config.instrument);
    println!("  • Silence Threshold: {:.1} dBFS", config.threshold_dbfs);
    println!("  • Pitch Mode:        {:?}", config.mode);
    println!("  • Hardware Delegate: {:?}", config.hardware);
    println!("\n(Tip: Pass `--help` to see all available CLI flags and examples)");

    // Enable live telemetry metrics in global settings
    let show_metrics = true;
    let device = config.hardware;
    let mode = match config.mode {
        PitchDetectorMode::Basic => HybridPitchDetectorMode::Mpm,
        PitchDetectorMode::Crnn => HybridPitchDetectorMode::Crnn,
        PitchDetectorMode::Hybrid => HybridPitchDetectorMode::Mpm,
    };

    let global_settings = Arc::new(Mutex::new(GlobalSettings::new(device, show_metrics, mode)));

    let error_cb = Arc::new(ConsoleErrorCallback);
    let (note_tx, note_rx) = RingBuffer::<Notes>::new(NOTE_RINGBUF_CAPACITY);

    let mut dsp = Dsp::new(
        config.instrument,
        config.threshold_dbfs,
        error_cb,
        config.mode,
        "".to_string(),
        "".to_string(),
        config.hardware,
        note_tx,
        global_settings,
    );

    if let Some(dev_name) = &config.device {
        dsp.set_audio_device(Some(dev_name.clone()));
    }

    let metrics = &*GLOBAL_AUDIO_METRICS;
    let running = Arc::new(AtomicBool::new(true));

    // Spawn background consumer outputter & live telemetry thread
    let running_clone = Arc::clone(&running);
    let consumer_handle = thread::spawn(move || {
        run_consumer_outputter(note_rx, metrics, running_clone);
    });

    // Start Audio Engine Stream
    println!("\nStarting audio processing stream (Press ENTER or Ctrl+C to stop)...");
    dsp.start::<HybridFeatureExtractor>()?;

    // Wait for user input to exit
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    println!("\nStopping audio engine...");
    dsp.stop();
    running.store(false, Ordering::Relaxed);
    let _ = consumer_handle.join();

    println!("Audio stream gracefully closed.");
    Ok(())
}
