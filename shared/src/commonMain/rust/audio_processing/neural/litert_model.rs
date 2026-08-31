//! # Hardware Device to Optimal Thread Count Mapping & Delegate Guide
//!
//! ### Hardware Device Thread Mapping:
//! | Hardware Device / Processor Type | Examples | Recommended Threads | Rationale |
//! | :--- | :--- | :---: | :--- |
//! | **Apple Neural Engine (NPU)** | Apple A12–A18, M1–M4 | **1** (or 2) | Offloaded entirely to ANE coprocessor. CPU only dispatches the task. |
//! | **Mobile GPU** | Qualcomm Adreno (6xx/7xx/8xx), ARM Mali-G, Apple Metal GPU | **1 – 2** | Shader kernels are queued from a single CPU host thread. |
//! | **Flagship ARM big.LITTLE / DynamIQ CPU** | Snapdragon 8 Gen 1–4, Google Tensor, Dimensity 9000/9400 | **2 – 4** | Matches the Performance (P) core cluster without spilling onto slow Efficiency (E) cores. |
//! | **Apple Silicon CPU (CPU-only mode)** | Apple A-series / M-series Performance Cores | **2 – 4** | 2–4 threads match the high-performance core cluster (Firestorm/Avalanche/Everest). |
//! | **Budget / Older ARM CPU** | Quad-core Cortex-A53 / Cortex-A55, ARMv7 | **2** | Prevents CPU starvation, thermal throttling, and real-time audio buffer underruns. |
//! | **Desktop x86_64 CPU** | Intel Core i5/i7/i9, AMD Ryzen (AVX2 / AVX-512) | **2 – 4** | High-throughput SIMD vector execution with minimal CPU usage. |
//! | **Android Studio x86 / x86_64 Emulator** | Virtualized Host CPU | **2** | Balances hypervisor virtualization overhead and host thread sharing. |
//!
//! ### What `HardwareDelegate::Auto` Does:
//! `HardwareDelegate::Auto` implements a hierarchical fallback chain at runtime:
//! 1. **NPU First**: Attempts to attach Apple Neural Engine / dedicated NPU.
//! 2. **GPU Fallback**: If NPU is unavailable or fails, falls back to Mobile GPU (Metal / OpenGL ES).
//! 3. **CPU SIMD Fallback**: If GPU is unavailable or fails, falls back to CPU SIMD execution (XNNPACK).

use crate::audio_processing::neural::crnn::{BasicPitchOutput, NeuralTranscriber};
use std::path::Path;

#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    clippy::all
)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/litert_bindings.rs"));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerMode {
    #[default]
    Battery,
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwareDelegate {
    #[default]
    Cpu,
    Gpu,
    Npu,
    Auto,
}

impl HardwareDelegate {
    /// Returns the recommended thread count for the selected hardware delegate.
    pub fn recommended_threads(&self, power_mode: PowerMode) -> i32 {
        match self {
            HardwareDelegate::Cpu => match power_mode {
                PowerMode::Battery => 2,
                PowerMode::Performance => 4,
            },
            HardwareDelegate::Gpu => 2,
            HardwareDelegate::Npu => 1,
            HardwareDelegate::Auto => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteRtError {
    NullPointer(String),
    AllocationFailed,
    InvocationFailed(i32),
    TensorCopyFailed,
    InvalidDimensions,
    InvalidPath(String),
    DelegateCreationFailed(String),
    UnsupportedPlatform,
}

impl std::fmt::Display for LiteRtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteRtError::NullPointer(msg) => write!(f, "LiteRT Null Pointer: {msg}"),
            LiteRtError::AllocationFailed => write!(f, "LiteRT Failed to allocate tensors"),
            LiteRtError::InvocationFailed(code) => {
                write!(f, "LiteRT Invocation failed with status code: {code}")
            }
            LiteRtError::TensorCopyFailed => {
                write!(f, "LiteRT Failed to copy data to/from tensor buffer")
            }
            LiteRtError::InvalidDimensions => write!(f, "LiteRT Invalid tensor dimensions"),
            LiteRtError::InvalidPath(p) => write!(f, "LiteRT Invalid model path: {p}"),
            LiteRtError::DelegateCreationFailed(d) => {
                write!(f, "LiteRT Failed to create delegate: {d}")
            }
            LiteRtError::UnsupportedPlatform => {
                write!(f, "LiteRT C library is not linked on this host target")
            }
        }
    }
}

impl std::error::Error for LiteRtError {}

#[cfg(all(
    any(target_os = "android", target_os = "ios"),
    any(target_arch = "aarch64", target_arch = "arm")
))]
mod platform_impl {
    use super::*;
    use std::ffi::{CString, c_void};
    use std::ptr::NonNull;

    pub struct SafeLiteRtModel {
        raw: NonNull<ffi::TfLiteModel>,
    }

    impl SafeLiteRtModel {
        pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, LiteRtError> {
            let path_str = path
                .as_ref()
                .to_str()
                .ok_or_else(|| LiteRtError::InvalidPath("Invalid UTF-8 in path".to_string()))?;
            let c_path = CString::new(path_str)
                .map_err(|_| LiteRtError::InvalidPath("Path contained null byte".to_string()))?;

            let model_ptr = unsafe { ffi::TfLiteModelCreateFromFile(c_path.as_ptr()) };
            let raw = NonNull::new(model_ptr).ok_or_else(|| {
                LiteRtError::NullPointer(format!("Failed to load model from file: {path_str}"))
            })?;

            Ok(Self { raw })
        }

        pub fn from_bytes(bytes: &[u8]) -> Result<Self, LiteRtError> {
            let model_ptr =
                unsafe { ffi::TfLiteModelCreate(bytes.as_ptr() as *const c_void, bytes.len()) };
            let raw = NonNull::new(model_ptr).ok_or_else(|| {
                LiteRtError::NullPointer("Failed to load model from byte buffer".to_string())
            })?;

            Ok(Self { raw })
        }

        pub fn as_raw(&self) -> *const ffi::TfLiteModel {
            self.raw.as_ptr()
        }
    }

    impl Drop for SafeLiteRtModel {
        fn drop(&mut self) {
            unsafe {
                ffi::TfLiteModelDelete(self.raw.as_ptr());
            }
        }
    }

    unsafe impl Send for SafeLiteRtModel {}
    unsafe impl Sync for SafeLiteRtModel {}

    pub struct SafeInterpreterOptions {
        raw: NonNull<ffi::TfLiteInterpreterOptions>,
    }

    impl SafeInterpreterOptions {
        pub fn new() -> Result<Self, LiteRtError> {
            let opts_ptr = unsafe { ffi::TfLiteInterpreterOptionsCreate() };
            let raw = NonNull::new(opts_ptr).ok_or_else(|| {
                LiteRtError::NullPointer("Failed to create interpreter options".to_string())
            })?;
            Ok(Self { raw })
        }

        pub fn set_num_threads(&mut self, num_threads: i32) {
            unsafe {
                ffi::TfLiteInterpreterOptionsSetNumThreads(self.raw.as_ptr(), num_threads);
            }
        }

        pub unsafe fn add_delegate(&mut self, delegate: *mut ffi::TfLiteOpaqueDelegate) {
            unsafe {
                ffi::TfLiteInterpreterOptionsAddDelegate(self.raw.as_ptr(), delegate);
            }
        }

        pub fn enable_gpu(&mut self) -> Result<(), LiteRtError> {
            let mut gpu_opts = unsafe { ffi::TfLiteGpuDelegateOptionsV2Default() };
            gpu_opts.is_precision_loss_allowed = 1;
            gpu_opts.inference_preference =
                ffi::TfLiteGpuInferencePreference_TFLITE_GPU_INFERENCE_PREFERENCE_FAST_SINGLE_ANSWER
                    as i32;

            let delegate = unsafe { ffi::TfLiteGpuDelegateV2Create(&gpu_opts) };
            if delegate.is_null() {
                return Err(LiteRtError::DelegateCreationFailed(
                    "GPU delegate creation returned null".into(),
                ));
            }
            unsafe {
                self.add_delegate(delegate);
            }
            Ok(())
        }

        #[cfg(target_os = "ios")]
        pub fn enable_npu(&mut self) -> Result<(), LiteRtError> {
            let coreml_opts = ffi::TfLiteCoreMlDelegateOptions {
                enabled_devices: ffi::TfLiteCoreMlDelegateEnabledDevices_TfLiteCoreMlDelegateDevicesNeuralEngineOnly,
                coreml_version: 3,
                max_delegated_partitions: 0,
                min_nodes_per_partition: 2,
            };
            let delegate = unsafe { ffi::TfLiteCoreMlDelegateCreate(&coreml_opts) };
            if delegate.is_null() {
                return Err(LiteRtError::DelegateCreationFailed(
                    "CoreML Neural Engine delegate creation returned null".into(),
                ));
            }
            unsafe {
                self.add_delegate(delegate);
            }
            Ok(())
        }

        #[cfg(not(target_os = "ios"))]
        pub fn enable_npu(&mut self) -> Result<(), LiteRtError> {
            self.enable_gpu()
        }

        pub fn enable_xnnpack(&mut self, num_threads: i32) -> Result<(), LiteRtError> {
            let mut xnnpack_opts = unsafe { ffi::TfLiteXNNPackDelegateOptionsDefault() };
            xnnpack_opts.num_threads = num_threads;
            let delegate = unsafe { ffi::TfLiteXNNPackDelegateCreate(&xnnpack_opts) };
            if delegate.is_null() {
                return Err(LiteRtError::DelegateCreationFailed(
                    "XNNPACK delegate creation returned null".into(),
                ));
            }
            unsafe {
                self.add_delegate(delegate);
            }
            Ok(())
        }

        pub fn enable_delegate(
            &mut self,
            delegate: HardwareDelegate,
            num_threads: i32,
        ) -> Result<(), LiteRtError> {
            match delegate {
                HardwareDelegate::Cpu => {
                    self.set_num_threads(num_threads);
                    let _ = self.enable_xnnpack(num_threads);
                    Ok(())
                }
                HardwareDelegate::Gpu => {
                    self.set_num_threads(num_threads);
                    self.enable_gpu()
                }
                HardwareDelegate::Npu => {
                    self.set_num_threads(num_threads);
                    self.enable_npu()
                }
                HardwareDelegate::Auto => {
                    self.set_num_threads(num_threads);
                    if self.enable_npu().is_err() && self.enable_gpu().is_err() {
                        let _ = self.enable_xnnpack(num_threads);
                    }
                    Ok(())
                }
            }
        }

        pub fn as_raw(&self) -> *const ffi::TfLiteInterpreterOptions {
            self.raw.as_ptr()
        }
    }

    impl Default for SafeInterpreterOptions {
        fn default() -> Self {
            Self::new().expect("Failed to initialize default SafeInterpreterOptions")
        }
    }

    impl Drop for SafeInterpreterOptions {
        fn drop(&mut self) {
            unsafe {
                ffi::TfLiteInterpreterOptionsDelete(self.raw.as_ptr());
            }
        }
    }

    pub struct SafeLiteRtInterpreter {
        raw: NonNull<ffi::TfLiteInterpreter>,
        _model: Option<SafeLiteRtModel>,
    }

    impl SafeLiteRtInterpreter {
        pub fn create(
            model: SafeLiteRtModel,
            options: Option<&SafeInterpreterOptions>,
        ) -> Result<Self, LiteRtError> {
            let opts_ptr = options.map_or(std::ptr::null(), |o| o.as_raw());
            let interp_ptr = unsafe { ffi::TfLiteInterpreterCreate(model.as_raw(), opts_ptr) };
            let raw = NonNull::new(interp_ptr).ok_or_else(|| {
                LiteRtError::NullPointer("Failed to create LiteRT Interpreter".to_string())
            })?;

            let mut interpreter = Self {
                raw,
                _model: Some(model),
            };

            interpreter.allocate_tensors()?;
            Ok(interpreter)
        }

        pub fn allocate_tensors(&mut self) -> Result<(), LiteRtError> {
            let status = unsafe { ffi::TfLiteInterpreterAllocateTensors(self.raw.as_ptr()) };
            if status == ffi::TfLiteStatus_kTfLiteOk {
                Ok(())
            } else {
                Err(LiteRtError::AllocationFailed)
            }
        }

        pub fn resize_input_tensor(
            &mut self,
            input_index: i32,
            dims: &[i32],
        ) -> Result<(), LiteRtError> {
            let status = unsafe {
                ffi::TfLiteInterpreterResizeInputTensor(
                    self.raw.as_ptr(),
                    input_index,
                    dims.as_ptr(),
                    dims.len() as i32,
                )
            };
            if status == ffi::TfLiteStatus_kTfLiteOk {
                self.allocate_tensors()
            } else {
                Err(LiteRtError::InvalidDimensions)
            }
        }

        pub fn set_input_data<T: Copy>(
            &mut self,
            input_index: i32,
            data: &[T],
        ) -> Result<(), LiteRtError> {
            let tensor =
                unsafe { ffi::TfLiteInterpreterGetInputTensor(self.raw.as_ptr(), input_index) };
            if tensor.is_null() {
                return Err(LiteRtError::NullPointer(format!(
                    "Input tensor at index {input_index} is null"
                )));
            }

            let byte_size = data.len() * std::mem::size_of::<T>();
            let status = unsafe {
                ffi::TfLiteTensorCopyFromBuffer(tensor, data.as_ptr() as *const c_void, byte_size)
            };

            if status == ffi::TfLiteStatus_kTfLiteOk {
                Ok(())
            } else {
                Err(LiteRtError::TensorCopyFailed)
            }
        }

        pub fn invoke(&mut self) -> Result<(), LiteRtError> {
            let status = unsafe { ffi::TfLiteInterpreterInvoke(self.raw.as_ptr()) };
            if status == ffi::TfLiteStatus_kTfLiteOk {
                Ok(())
            } else {
                Err(LiteRtError::InvocationFailed(status as i32))
            }
        }

        pub fn get_output_data<T: Copy>(
            &self,
            output_index: i32,
            dest: &mut [T],
        ) -> Result<(), LiteRtError> {
            let tensor =
                unsafe { ffi::TfLiteInterpreterGetOutputTensor(self.raw.as_ptr(), output_index) };
            if tensor.is_null() {
                return Err(LiteRtError::NullPointer(format!(
                    "Output tensor at index {output_index} is null"
                )));
            }

            let byte_size = dest.len() * std::mem::size_of::<T>();
            let status = unsafe {
                ffi::TfLiteTensorCopyToBuffer(tensor, dest.as_mut_ptr() as *mut c_void, byte_size)
            };

            if status == ffi::TfLiteStatus_kTfLiteOk {
                Ok(())
            } else {
                Err(LiteRtError::TensorCopyFailed)
            }
        }

        pub fn input_tensor_count(&self) -> i32 {
            unsafe { ffi::TfLiteInterpreterGetInputTensorCount(self.raw.as_ptr()) }
        }

        pub fn input_tensor_byte_size(&self, input_index: i32) -> usize {
            let tensor =
                unsafe { ffi::TfLiteInterpreterGetInputTensor(self.raw.as_ptr(), input_index) };
            if tensor.is_null() {
                0
            } else {
                unsafe { ffi::TfLiteTensorByteSize(tensor) }
            }
        }

        pub fn output_tensor_count(&self) -> i32 {
            unsafe { ffi::TfLiteInterpreterGetOutputTensorCount(self.raw.as_ptr()) }
        }
    }

    impl Drop for SafeLiteRtInterpreter {
        fn drop(&mut self) {
            unsafe {
                ffi::TfLiteInterpreterDelete(self.raw.as_ptr());
            }
        }
    }

    unsafe impl Send for SafeLiteRtInterpreter {}
}

#[cfg(not(all(
    any(target_os = "android", target_os = "ios"),
    any(target_arch = "aarch64", target_arch = "arm")
)))]
mod platform_impl {
    use super::*;

    #[derive(Debug, Default)]
    pub struct SafeLiteRtModel;

    impl SafeLiteRtModel {
        pub fn from_file<P: AsRef<Path>>(_path: P) -> Result<Self, LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn from_bytes(_bytes: &[u8]) -> Result<Self, LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn as_raw(&self) -> *const ffi::TfLiteModel {
            std::ptr::null()
        }
    }

    #[derive(Debug, Default)]
    pub struct SafeInterpreterOptions;

    impl SafeInterpreterOptions {
        pub fn new() -> Result<Self, LiteRtError> {
            Ok(Self)
        }

        pub fn set_num_threads(&mut self, _num_threads: i32) {}

        pub unsafe fn add_delegate(&mut self, _delegate: *mut ffi::TfLiteOpaqueDelegate) {}

        pub fn enable_gpu(&mut self) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn enable_npu(&mut self) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn enable_xnnpack(&mut self, _num_threads: i32) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn enable_delegate(
            &mut self,
            _delegate: HardwareDelegate,
            _num_threads: i32,
        ) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn as_raw(&self) -> *const ffi::TfLiteInterpreterOptions {
            std::ptr::null()
        }
    }

    #[derive(Debug, Default)]
    pub struct SafeLiteRtInterpreter;

    impl SafeLiteRtInterpreter {
        pub fn create(
            _model: SafeLiteRtModel,
            _options: Option<&SafeInterpreterOptions>,
        ) -> Result<Self, LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn allocate_tensors(&mut self) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn resize_input_tensor(
            &mut self,
            _input_index: i32,
            _dims: &[i32],
        ) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn set_input_data<T: Copy>(
            &mut self,
            _input_index: i32,
            _data: &[T],
        ) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn invoke(&mut self) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn get_output_data<T: Copy>(
            &self,
            _output_index: i32,
            _dest: &mut [T],
        ) -> Result<(), LiteRtError> {
            Err(LiteRtError::UnsupportedPlatform)
        }

        pub fn input_tensor_count(&self) -> i32 {
            0
        }

        pub fn input_tensor_byte_size(&self, _input_index: i32) -> usize {
            0
        }

        pub fn output_tensor_count(&self) -> i32 {
            0
        }
    }

    unsafe impl Send for SafeLiteRtInterpreter {}
}

pub use platform_impl::*;

pub struct LiteRtBasicPitchModel {
    sample_rate: u32,
    interpreter: Option<SafeLiteRtInterpreter>,
}

impl LiteRtBasicPitchModel {
    pub fn from_file<P: AsRef<Path>>(
        model_path: P,
        delegate: HardwareDelegate,
        num_threads: i32,
    ) -> Result<Self, LiteRtError> {
        let model = SafeLiteRtModel::from_file(model_path)?;
        let mut options = SafeInterpreterOptions::new()?;
        options.enable_delegate(delegate, num_threads)?;

        let interpreter = SafeLiteRtInterpreter::create(model, Some(&options))?;
        Ok(Self {
            sample_rate: 22050,
            interpreter: Some(interpreter),
        })
    }

    pub fn from_bytes(
        bytes: &[u8],
        delegate: HardwareDelegate,
        num_threads: i32,
    ) -> Result<Self, LiteRtError> {
        let model = SafeLiteRtModel::from_bytes(bytes)?;
        let mut options = SafeInterpreterOptions::new()?;
        options.enable_delegate(delegate, num_threads)?;

        let interpreter = SafeLiteRtInterpreter::create(model, Some(&options))?;
        Ok(Self {
            sample_rate: 22050,
            interpreter: Some(interpreter),
        })
    }

    pub fn new_stub() -> Self {
        Self {
            sample_rate: 22050,
            interpreter: None,
        }
    }
}

impl NeuralTranscriber for LiteRtBasicPitchModel {
    fn transcribe_hop(&mut self, audio_hop_22k: &[f32], loudness: f32) -> BasicPitchOutput {
        let mut out = BasicPitchOutput::default();
        out.energy_dbfs = loudness;

        if let Some(interpreter) = &mut self.interpreter {
            let expected_bytes = interpreter.input_tensor_byte_size(0);
            let expected_samples = if expected_bytes > 0 {
                expected_bytes / std::mem::size_of::<f32>()
            } else {
                audio_hop_22k.len()
            };

            let status = if expected_samples == audio_hop_22k.len() {
                interpreter.set_input_data(0, audio_hop_22k)
            } else if expected_samples > 0 && expected_samples != audio_hop_22k.len() {
                // If model has a fixed input tensor dimension and resampled output has slight variance,
                // safely pad or slice into a static stack buffer to match exact tensor dimensions without heap allocation
                let mut padded = [0.0f32; 1024];
                if expected_samples <= padded.len() {
                    let copy_len = audio_hop_22k.len().min(expected_samples);
                    padded[..copy_len].copy_from_slice(&audio_hop_22k[..copy_len]);
                    interpreter.set_input_data(0, &padded[..expected_samples])
                } else {
                    interpreter.set_input_data(0, audio_hop_22k)
                }
            } else {
                interpreter.set_input_data(0, audio_hop_22k)
            };

            if status.is_ok() && interpreter.invoke().is_ok() {
                let _ = interpreter.get_output_data(0, &mut out.onsets);
                let _ = interpreter.get_output_data(1, &mut out.frames);
                let _ = interpreter.get_output_data(2, &mut out.contours);
            }
        } else {
            out.onsets.fill(0.0);
            out.frames.fill(0.0);
            out.contours.fill(0.0);
        }

        out
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_litert_stub_creation() {
        let mut model = LiteRtBasicPitchModel::new_stub();
        assert_eq!(model.sample_rate(), 22050);

        let audio = [0.0f32; 512];
        let output = model.transcribe_hop(audio.as_slice(), -20.0);
        assert_eq!(output.energy_dbfs, -20.0);
        assert_eq!(output.onsets[0], 0.0);
        assert_eq!(output.frames[0], 0.0);
    }

    #[test]
    fn test_recommended_threads() {
        assert_eq!(
            HardwareDelegate::Cpu.recommended_threads(PowerMode::Battery),
            2
        );
        assert_eq!(
            HardwareDelegate::Cpu.recommended_threads(PowerMode::Performance),
            4
        );
        assert_eq!(
            HardwareDelegate::Gpu.recommended_threads(PowerMode::Battery),
            2
        );
        assert_eq!(
            HardwareDelegate::Npu.recommended_threads(PowerMode::Battery),
            1
        );
        assert_eq!(
            HardwareDelegate::Auto.recommended_threads(PowerMode::Battery),
            2
        );
    }
}
