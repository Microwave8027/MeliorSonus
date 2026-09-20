# MeliorSonus Neural Transcriber: ByteDance CRNN Export Pipeline

Exports the ByteDance CRNN piano transcription architecture (Kong et al. 2021) into:
- **ONNX**: FP32 and INT8 for native Rust inference via Tract-ONNX (`tract-onnx`) and ONNX Runtime.
- **LiteRT / TFLite**: FP32 and Dynamic Range INT8 (DRQ) via Google's `ai-edge-torch`.

The exported models output 7 heads matching `shared/src/commonMain/rust/audio_processing/neural/bytedance_crnn.rs`:
1. `note_onsets`: `[batch, 88]` (Transient onset detection)
2. `note_offsets`: `[batch, 88]` (Transient note release detection)
3. `note_frames`: `[batch, 88]` (Sustained note presence)
4. `note_velocity`: `[batch, 88]` (Physical strike velocity regression in range `[0, 128]`)
5. `pedal_onset`: `[batch, 1]` (Sustain pedal onset event)
6. `pedal_offset`: `[batch, 1]` (Sustain pedal release event)
7. `pedal_frame`: `[batch, 1]` (Sustain pedal active frame)

---

## Which Models Should I Use in MeliorSonus?

All exported models are stored in [`tools/export_crnn/models/`](./models/):

| Target Platform / Engine | Recommended File | File Size | Why |
| :--- | :--- | :--- | :--- |
| **Android & iOS (LiteRT / TFLite)** | **`bytedance_crnn_acoustic_drq_int8.tflite`** | **3.66 MB** | **Recommended for Mobile.** ~70% smaller memory footprint, fast L1/L2 cache hits, and hardware-accelerated by LiteRT CPU (XNNPACK). |
| **Desktop (Windows, macOS, Linux with Tract)** | **`bytedance_crnn_acoustic_fp32.onnx`** | **12.4 MB** | **Recommended for Desktop.** MeliorSonus uses pure-Rust `tract-onnx` on desktop, executing without requiring any external C++ shared libraries. |

### Other Exported Models & When to Use Them:

- **`bytedance_crnn_acoustic_fp32.tflite`** (12.8 MB):
  Use if you are profiling mobile GPU delegates (OpenCL / Metal / Vulkan) that require or prefer full FP32/FP16 precision over INT8.
- **`bytedance_crnn_acoustic_int8.onnx`** (10.5 MB):
  Use only if you are running Microsoft ONNX Runtime (ORT) with dynamic INT8 support instead of pure-Rust Tract.
- **`bytedance_crnn_end_to_end_fp32.onnx`** (30.1 MB):
  Includes the in-graph Mel-spectrogram filterbank (raw 16 kHz PCM -> CRNN). Not required for real-time MeliorSonus audio streaming because the Rust audio DSP frontend (`SlaneyMelFrontend` in `mel_spectrogram.rs`) already computes the 229-bin Slaney Log-Mel spectrogram natively with zero memory allocations.

### Integration in Code (Rust & Kotlin):

When configuring model paths in MeliorSonus:
```rust
// In Rust (audio_processing::dsp::dsp):
set_bytedance_model_paths(
    "models/bytedance_crnn_acoustic_drq_int8.tflite".to_string(), // Primary LiteRT (Mobile)
    "models/bytedance_crnn_acoustic_fp32.onnx".to_string(),       // Tract-ONNX fallback (Desktop)
);
```

---

## Windows Building & Platform Notes

### Why `ai-edge-torch` requires WSL2 on Windows
Google's `ai-edge-torch` plugin compiles PyTorch graphs to LiteRT using OpenXLA / StableHLO (`torch_xla`).
`torch_xla` has official prebuilt wheels only for **Linux** and **macOS**; there are no native Windows wheels or MSVC build targets.

- **On Windows**:
  - Running directly in Windows (`python export_bytedance_crnn.py`) will export the **FP32 ONNX model** (and optional INT8 ONNX model). These ONNX models are directly compatible with MeliorSonus's pure-Rust `tract-onnx` inference engine.
  - To export **LiteRT / TFLite** models via `ai-edge-torch`, run via the automated WSL2 script:
    ```cmd
    run_wsl_export.bat
    ```
    Or via the CLI bridge:
    ```powershell
    python export_bytedance_crnn.py --wsl
    ```
    This automatically executes the export inside the local WSL2 Linux virtual environment and writes the models back to `./models/`.

- **On Linux / macOS**:
  - `ai-edge-torch` runs natively and produces all ONNX and LiteRT/TFLite models directly:
    ```bash
    python export_bytedance_crnn.py
    ```

---

## CLI Options

### 1. Acoustic Model (Default, Recommended for MeliorSonus)
```bash
python export_bytedance_crnn.py --checkpoint /path/to/note_and_pedal_crnn.pth
```

### 2. End-to-End Model (In-graph Log-Mel Spectrogram)
Embeds the differentiable Conv1d DFT filterbank inside the graph taking raw 16 kHz audio PCM:
```bash
python export_bytedance_crnn.py --end-to-end --checkpoint /path/to/note_and_pedal_crnn.pth
```

### 3. Full Sequence Transcription (Batch Mode)
Exports the model to predict note states across all time steps `[batch, time_steps, 88]` instead of just the latest frame:
```bash
python export_bytedance_crnn.py --sequence --checkpoint /path/to/note_and_pedal_crnn.pth
```

### 4. Zero-Lookahead Unidirectional Streaming
```bash
python export_bytedance_crnn.py --unidirectional --checkpoint /path/to/note_and_pedal_crnn.pth
```

---

## Output Artifact Summary

Stored in `./models/`:
- `bytedance_crnn_acoustic_drq_int8.tflite`: **3.66 MB** (Primary Mobile Model)
- `bytedance_crnn_acoustic_fp32.tflite`: **12.8 MB** (Unquantized LiteRT Model)
- `bytedance_crnn_acoustic_fp32.onnx`: **12.4 MB** (Primary Desktop Model for Tract)
- `bytedance_crnn_acoustic_int8.onnx`: **10.5 MB** (Dynamic Quantized ONNX Model)
- `bytedance_crnn_end_to_end_fp32.onnx`: **30.1 MB** (End-to-End Audio PCM Model)
