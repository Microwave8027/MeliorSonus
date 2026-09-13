uniffi::setup_scaffolding!();

use compose_app::audio_processing::DEFAULT_SILENCE_THRESHOLD_DBFS;
use compose_app::audio_processing::HybridFeatureExtractor;
use compose_app::audio_processing::Instrument;
use compose_app::audio_processing::{Notes, PitchDetectorMode, WavReader};
use compose_app::utils::ErrorCallback;
use compose_app::utils::GlobalSettings;
use compose_app::utils::errors::RustError;
use rtrb::Consumer;
use std::path::Path;
use std::sync::{Arc, Mutex};

mod downloader;

#[uniffi::export(callback_interface)]
pub trait CapturerCallback: Send + Sync + 'static {
    fn on_error(&self, msg: String);
    fn on_complete(&self);
}

struct ErrorCallbackBridge(Box<dyn CapturerCallback>);

impl ErrorCallback for ErrorCallbackBridge {
    fn on_error(&self, msg: compose_app::utils::errors::RustError) {
        self.0.on_error(msg.to_string());
    }
    fn on_complete(&self) {
        self.0.on_complete();
    }
}

#[uniffi::export]
pub fn set_onnx_model_path(path: String) {
    let _ = compose_app::audio_processing::ONNX_MODEL_PATH.set(path);
}

#[derive(uniffi::Object)]
pub struct AudioCapturer {
    rb_cons: Mutex<Consumer<Notes>>,
    wav: Mutex<WavReader>,
}

#[uniffi::export]
impl AudioCapturer {
    #[uniffi::constructor]
    pub fn new(path: String, err_cb: Box<dyn CapturerCallback>) -> Self {
        let (prod, cons) = rtrb::RingBuffer::new(400);

        if compose_app::audio_processing::ONNX_MODEL_PATH
            .get()
            .is_none()
        {
            let candidates = [
                "src/commonMain/rust/assets/basic_pitch_models/nmp.onnx",
                "shared/src/commonMain/rust/assets/basic_pitch_models/nmp.onnx",
                "../src/commonMain/rust/assets/basic_pitch_models/nmp.onnx",
                "../../src/commonMain/rust/assets/basic_pitch_models/nmp.onnx",
            ];
            for cand in candidates {
                if Path::new(cand).exists() {
                    let _ = compose_app::audio_processing::ONNX_MODEL_PATH.set(cand.to_string());
                    break;
                }
            }
        }

        let path_buf = Path::new(&path);
        let hound = WavReader::new(
            path_buf,
            Instrument::Piano,
            Arc::new(ErrorCallbackBridge(err_cb)),
            prod,
            DEFAULT_SILENCE_THRESHOLD_DBFS,
            PitchDetectorMode::Crnn,
            Arc::new(Mutex::new(GlobalSettings::default())),
        );
        Self {
            rb_cons: Mutex::new(cons),
            wav: Mutex::new(hound),
        }
    }

    pub fn play(&self) -> Result<(), RustError> {
        if let Ok(mut wav) = self.wav.lock() {
            if let Err(err) = wav.play::<HybridFeatureExtractor>() {
                return Err(RustError::Custom(err.to_string()));
            }
        }
        Ok(())
    }

    pub fn get_notes(&self) -> Option<Notes> {
        if let Ok(mut prod) = self.rb_cons.lock() {
            prod.pop().ok()
        } else {
            None
        }
    }

    pub fn reset(&self) {
        if let Ok(mut c) = self.rb_cons.lock()
            && let Ok(mut wav) = self.wav.lock()
        {
            let (prod, cons) = rtrb::RingBuffer::new(400);
            *c = cons;
            wav.reset(prod);
        }
    }

    pub fn end(&self) {
        if let Ok(mut wav) = self.wav.lock() {
            wav.end();
        }
    }
}
