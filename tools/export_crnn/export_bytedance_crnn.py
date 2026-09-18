"""
MeliorSonus Neural Transcriber Tooling: ByteDance CRNN Export & Quantization Pipeline
Exports the ByteDance CRNN piano transcription model (Kong et al.) with all 7 output heads:
  1. Note Onsets: [batch, 88]
  2. Note Offsets: [batch, 88]
  3. Note Frames (sustain): [batch, 88]
  4. Note Velocity (strike dynamics): [batch, 88]
  5. Pedal Onset: [batch, 1] (scalar)
  6. Pedal Offset: [batch, 1] (scalar)
  7. Pedal Frame: [batch, 1] (scalar)

Quantization Backends:
  - ONNX: FP32 and INT8 for Tract-ONNX in pure Rust and ONNX Runtime.
  - TFLite / LiteRT: FP32, FP16, Dynamic Range INT8, and Full Integer PTQ via Google's `ai-edge-torch`.
    NOTE: Google's `ai-edge-torch` requires `torch_xla` (StableHLO/OpenXLA), which is supported on
    Linux and macOS. On Windows, LiteRT export runs seamlessly via WSL2 (use `--wsl` flag or `run_wsl_export.bat`).
"""

import argparse
import os
import sys

# Prevent protobuf duplicate symbol abort between torch_xla and tensorflow
os.environ["PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION"] = "python"
import numpy as np
import torch
import torch.nn as nn

# Ensure safe console output on Windows (prevent charmap/cp1252 crash on Unicode symbols)
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
if hasattr(sys.stderr, "reconfigure"):
    try:
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

IS_WINDOWS = sys.platform == "win32"

# Safe import for LiteRT / TFLite compiler (formerly ai_edge_torch, now litert_torch)
AI_EDGE_TORCH_STATUS = "ok"
AI_EDGE_TORCH_ERR = None
HAS_AI_EDGE_TORCH = False
HAS_AI_EDGE_QUANT = False
litert_module = None
QuantConfig = None

try:
    import litert_torch as litert_module
    HAS_AI_EDGE_TORCH = True
except (ImportError, AttributeError, Exception):
    try:
        import ai_edge_torch as litert_module
        if hasattr(litert_module, "convert"):
            HAS_AI_EDGE_TORCH = True
        else:
            HAS_AI_EDGE_TORCH = False
            AI_EDGE_TORCH_ERR = "ai_edge_torch is deprecated shim; install litert-torch"
    except (ImportError, AttributeError, Exception) as e:
        HAS_AI_EDGE_TORCH = False
        AI_EDGE_TORCH_ERR = str(e)
        if "torch_xla" in AI_EDGE_TORCH_ERR:
            AI_EDGE_TORCH_STATUS = "missing_torch_xla"
        elif "tensorflow" in AI_EDGE_TORCH_ERR:
            AI_EDGE_TORCH_STATUS = "missing_tensorflow"
        else:
            AI_EDGE_TORCH_STATUS = "not_installed"

if HAS_AI_EDGE_TORCH:
    try:
        from litert_torch.quantize.quant_config import QuantConfig
        from litert_torch.quantize.pt2e_quantizer import (
            PT2EQuantizer,
            get_symmetric_quantization_config,
        )
        from torch.ao.quantization.quantize_pt2e import prepare_pt2e, convert_pt2e
        HAS_AI_EDGE_QUANT = True
    except Exception:
        try:
            from ai_edge_torch.quantize.quant_config import QuantConfig
            from ai_edge_torch.quantize.pt2e_quantizer import (
                PT2EQuantizer,
                get_symmetric_quantization_config,
            )
            from torch.ao.quantization.quantize_pt2e import prepare_pt2e, convert_pt2e
            HAS_AI_EDGE_QUANT = True
        except Exception:
            HAS_AI_EDGE_QUANT = False

try:
    import onnx
    from onnxruntime.quantization import quantize_dynamic, QuantType
    HAS_ONNX_QUANT = True
except ImportError:
    HAS_ONNX_QUANT = False


# =====================================================================
# 1. ByteDance CRNN Model Architecture (Kong et al. 2021)
# =====================================================================
class ConvBlock(nn.Module):
    def __init__(self, in_channels, out_channels):
        super().__init__()
        # inplace=False is critical to allow clean graph tracing with torch.export / Dynamo
        self.conv1 = nn.Conv2d(in_channels, out_channels, kernel_size=3, padding=1, bias=False)
        self.bn1 = nn.BatchNorm2d(out_channels)
        self.relu1 = nn.ReLU(inplace=False)
        self.conv2 = nn.Conv2d(out_channels, out_channels, kernel_size=3, padding=1, bias=False)
        self.bn2 = nn.BatchNorm2d(out_channels)
        self.relu2 = nn.ReLU(inplace=False)

    def forward(self, x):
        x = self.relu1(self.bn1(self.conv1(x)))
        x = self.relu2(self.bn2(self.conv2(x)))
        return x


class ByteDanceCRNNAcoustic(nn.Module):
    """
    ByteDance CRNN core acoustic model predicting note and pedal states.
    Input: Log-Mel spectrogram [batch, 1, time_steps, freq_bins=229]
    Outputs (7 Tensors):
      0. note_onsets:   [batch, 88] (or [batch, time_steps, 88] if sequence_mode)
      1. note_offsets:  [batch, 88] (or [batch, time_steps, 88] if sequence_mode)
      2. note_frames:   [batch, 88] (or [batch, time_steps, 88] if sequence_mode)
      3. note_velocity: [batch, 88] (or [batch, time_steps, 88] if sequence_mode)
      4. pedal_onset:   [batch, 1]  (or [batch, time_steps, 1] if sequence_mode)
      5. pedal_offset:  [batch, 1]  (or [batch, time_steps, 1] if sequence_mode)
      6. pedal_frame:   [batch, 1]  (or [batch, time_steps, 1] if sequence_mode)
    """
    def __init__(self, num_pitches=88, hidden_size=256, bidirectional=True, sequence_mode=False):
        super().__init__()
        self.num_pitches = num_pitches
        self.hidden_size = hidden_size
        self.bidirectional = bidirectional
        self.sequence_mode = sequence_mode
        self.num_layers = 2

        self.conv1 = ConvBlock(1, 48)
        self.pool1 = nn.MaxPool2d(kernel_size=(1, 2))

        self.conv2 = ConvBlock(48, 64)
        self.pool2 = nn.MaxPool2d(kernel_size=(1, 2))

        self.conv3 = ConvBlock(64, 96)
        self.pool3 = nn.MaxPool2d(kernel_size=(1, 2))

        self.conv4 = ConvBlock(96, 128)
        self.pool4 = nn.MaxPool2d(kernel_size=(1, 2))

        # 229 freq bins reduced through 4 MaxPool2d(1, 2):
        # 229 -> 114 -> 57 -> 28 -> 14. 128 channels * 14 = 1792.
        conv_out_dim = 128 * 14
        self.fc_in = nn.Linear(conv_out_dim, hidden_size)

        self.gru = nn.GRU(
            input_size=hidden_size,
            hidden_size=hidden_size,
            num_layers=self.num_layers,
            batch_first=True,
            bidirectional=bidirectional,
        )
        gru_out_dim = hidden_size * (2 if bidirectional else 1)

        # 4 Note Heads:
        self.fc_onset = nn.Linear(gru_out_dim, num_pitches)
        self.fc_offset = nn.Linear(gru_out_dim, num_pitches)
        self.fc_frame = nn.Linear(gru_out_dim, num_pitches)
        self.fc_velocity = nn.Linear(gru_out_dim, num_pitches)

        # 3 Pedal Heads (1x1 scalars per frame):
        self.fc_pedal_onset = nn.Linear(gru_out_dim, 1)
        self.fc_pedal_offset = nn.Linear(gru_out_dim, 1)
        self.fc_pedal_frame = nn.Linear(gru_out_dim, 1)

    def forward(self, x):
        # x: [batch, 1, time_steps, 229]
        x = self.pool1(self.conv1(x))
        x = self.pool2(self.conv2(x))
        x = self.pool3(self.conv3(x))
        x = self.pool4(self.conv4(x))

        x = x.permute(0, 2, 1, 3)
        b, t, c, f = x.shape
        x = x.reshape(b, t, c * f)

        x = torch.relu(self.fc_in(x))
        gru_out, _ = self.gru(x)

        if self.sequence_mode:
            features = gru_out
        else:
            # Extract latest frame for real-time streaming audio pipeline
            features = gru_out[:, -1, :]

        note_onsets = torch.sigmoid(self.fc_onset(features))
        note_offsets = torch.sigmoid(self.fc_offset(features))
        note_frames = torch.sigmoid(self.fc_frame(features))
        # In Kong et al. 2021 ByteDance CRNN, velocity regression targets are in [0, 128]
        note_velocity = torch.clamp(self.fc_velocity(features), 0.0, 128.0)

        pedal_onset = torch.sigmoid(self.fc_pedal_onset(features))
        pedal_offset = torch.sigmoid(self.fc_pedal_offset(features))
        pedal_frame = torch.sigmoid(self.fc_pedal_frame(features))

        return (
            note_onsets,
            note_offsets,
            note_frames,
            note_velocity,
            pedal_onset,
            pedal_offset,
            pedal_frame,
        )


class ExportableMelSpectrogram(nn.Module):
    """
    ONNX- and LiteRT-compatible Differentiable Log-Mel Spectrogram frontend.
    Uses 1D Conv DFT basis kernels and precomputed Slaney Mel filterbank,
    avoiding complex STFT ops which break torch.onnx and LiteRT conversion.
    """
    def __init__(
        self,
        sample_rate: int = 16000,
        n_fft: int = 2048,
        hop_length: int = 160,
        n_mels: int = 229,
        f_min: float = 30.0,
        f_max: float = 8000.0,
    ):
        super().__init__()
        self.n_fft = n_fft
        self.hop_length = hop_length
        self.n_bins = n_fft // 2 + 1  # 1025 bins

        # 1. Precompute periodic Hann window
        window = torch.hann_window(n_fft, periodic=True)

        # 2. Precompute DFT basis kernels as Conv1d filters
        n = torch.arange(n_fft, dtype=torch.float32)
        k = torch.arange(self.n_bins, dtype=torch.float32).unsqueeze(1)
        angles = 2.0 * np.pi * k * n / n_fft

        real_basis = (torch.cos(angles) * window).unsqueeze(1)  # [1025, 1, 2048]
        imag_basis = (-torch.sin(angles) * window).unsqueeze(1) # [1025, 1, 2048]

        self.register_buffer("real_basis", real_basis)
        self.register_buffer("imag_basis", imag_basis)

        # 3. Slaney Mel filterbank
        try:
            import torchaudio.functional as F
            mel_fb = F.melscale_fbanks(
                n_freqs=self.n_bins,
                f_min=f_min,
                f_max=f_max,
                n_mels=n_mels,
                sample_rate=sample_rate,
                norm="slaney",
            )
        except Exception:
            # Fallback filterbank construction
            mel_fb = self._create_slaney_fb(sample_rate, n_fft, n_mels, f_min, f_max)

        self.register_buffer("mel_fb", mel_fb)

    def _create_slaney_fb(self, sr, n_fft, n_mels, f_min, f_max):
        def hz_to_mel(f):
            return 2595.0 * np.log10(1.0 + f / 700.0)
        def mel_to_hz(m):
            return 700.0 * (10.0 ** (m / 2595.0) - 1.0)

        m_pts = np.linspace(hz_to_mel(f_min), hz_to_mel(f_max), n_mels + 2)
        f_pts = mel_to_hz(m_pts)
        bins = np.floor((n_fft + 1) * f_pts / sr).astype(int)

        fb = torch.zeros(self.n_bins, n_mels, dtype=torch.float32)
        for m in range(1, n_mels + 1):
            f_m_minus = bins[m - 1]
            f_m = bins[m]
            f_m_plus = bins[m + 1]

            for k in range(f_m_minus, f_m):
                if k < self.n_bins:
                    fb[k, m - 1] = (k - f_m_minus) / max(f_m - f_m_minus, 1)
            for k in range(f_m, f_m_plus):
                if k < self.n_bins:
                    fb[k, m - 1] = (f_m_plus - k) / max(f_m_plus - f_m, 1)

            # Slaney normalization (area = 2 / (f_m_plus - f_m_minus))
            enorm = 2.0 / max(f_pts[m + 1] - f_pts[m - 1], 1e-6)
            fb[:, m - 1] *= enorm

        return fb

    def forward(self, audio_pcm: torch.Tensor) -> torch.Tensor:
        # audio_pcm: [batch, samples]
        x = audio_pcm.unsqueeze(1)  # [batch, 1, samples]
        pad = self.n_fft // 2
        x = torch.nn.functional.pad(x, (pad, pad), mode="reflect")
        real = torch.nn.functional.conv1d(x, self.real_basis, stride=self.hop_length)
        imag = torch.nn.functional.conv1d(x, self.imag_basis, stride=self.hop_length)
        power_spec = real.pow(2) + imag.pow(2)  # [batch, 1025, time]
        power_spec = power_spec.transpose(1, 2) # [batch, time, 1025]
        mel = torch.matmul(power_spec, self.mel_fb) # [batch, time, 229]
        log_mel = 10.0 * torch.log10(torch.clamp(mel, min=1e-10))
        return log_mel.unsqueeze(1)  # [batch, 1, time, 229]


class MeliorSonusCRNNEndToEnd(nn.Module):
    """
    End-to-End wrapper taking raw 16 kHz audio PCM tensor [batch, samples]
    and embedding the Log-Mel Spectrogram frontend directly in the computation graph.
    """
    def __init__(self, core_crnn):
        super().__init__()
        self.mel = ExportableMelSpectrogram(
            sample_rate=16000,
            n_fft=2048,
            hop_length=160,
            n_mels=229,
            f_min=30.0,
            f_max=8000.0,
        )
        self.crnn = core_crnn

    def forward(self, audio_pcm: torch.Tensor):
        log_mel = self.mel(audio_pcm)
        return self.crnn(log_mel)


# =====================================================================
# 2. Robust Checkpoint Loading & Remapping
# =====================================================================
def load_checkpoint(model: nn.Module, checkpoint_path: str) -> bool:
    """
    Loads checkpoint weights with automatic dict unwrapping and key remapping
    for Kong et al. 2021 ByteDance piano transcription checkpoints.
    """
    print(f"[*] Loading pretrained weights from {checkpoint_path}...")
    try:
        raw = torch.load(checkpoint_path, map_location="cpu")
    except Exception as e:
        print(f"[!] Error reading checkpoint file: {e}")
        return False

    if isinstance(raw, dict):
        if "model" in raw:
            state_dict = raw["model"]
        elif "state_dict" in raw:
            state_dict = raw["state_dict"]
        else:
            state_dict = raw
    else:
        state_dict = raw

    # Strip 'module.' prefix (from DataParallel)
    clean_dict = {}
    for k, v in state_dict.items():
        name = k[7:] if k.startswith("module.") else k
        clean_dict[name] = v

    # Automatic key remapping for Kong et al. official checkpoints:
    remap_rules = {
        "conv_block1.": "conv1.",
        "conv_block2.": "conv2.",
        "conv_block3.": "conv3.",
        "conv_block4.": "conv4.",
        "reg_onset_output.": "fc_onset.",
        "reg_offset_output.": "fc_offset.",
        "frame_output.": "fc_frame.",
        "reg_velocity_output.": "fc_velocity.",
        "reg_pedal_onset_output.": "fc_pedal_onset.",
        "reg_pedal_offset_output.": "fc_pedal_offset.",
        "pedal_frame_output.": "fc_pedal_frame.",
    }

    remapped_dict = {}
    for k, v in clean_dict.items():
        new_k = k
        for old_prefix, new_prefix in remap_rules.items():
            if old_prefix in new_k:
                new_k = new_k.replace(old_prefix, new_prefix)
        remapped_dict[new_k] = v

    model_dict = model.state_dict()
    matched_keys = [k for k in remapped_dict if k in model_dict and model_dict[k].shape == remapped_dict[k].shape]
    missing_keys = [k for k in model_dict if k not in remapped_dict]

    print(f"    -> Matched {len(matched_keys)} / {len(model_dict)} model parameters.")
    if missing_keys:
        print(f"    -> Info: {len(missing_keys)} model keys will use initial weights.")

    model.load_state_dict(remapped_dict, strict=False)
    return True


# =====================================================================
# 3. Export & Quantization Pipeline Functions
# =====================================================================
def export_onnx(model, output_dir, opset_version=17, acoustic_only=True, quantize=False):
    os.makedirs(output_dir, exist_ok=True)
    model.eval()

    if acoustic_only:
        dummy_input = torch.zeros((1, 1, 32, 229), dtype=torch.float32)
        input_name = "log_mel_input"
        dynamic_axes_input = {0: "batch_size", 2: "time_steps"}
        prefix = "bytedance_crnn_acoustic"
    else:
        dummy_input = torch.zeros((1, 16000), dtype=torch.float32)
        input_name = "audio_pcm"
        dynamic_axes_input = {0: "batch_size", 1: "num_samples"}
        prefix = "bytedance_crnn_end_to_end"

    fp32_onnx_path = os.path.join(output_dir, f"{prefix}_fp32.onnx")

    print(f"[*] Exporting FP32 ONNX model to {fp32_onnx_path}...")
    torch.onnx.export(
        model,
        dummy_input,
        fp32_onnx_path,
        export_params=True,
        opset_version=opset_version,
        do_constant_folding=True,
        input_names=[input_name],
        output_names=[
            "note_onsets",
            "note_offsets",
            "note_frames",
            "note_velocity",
            "pedal_onset",
            "pedal_offset",
            "pedal_frame",
        ],
        dynamic_axes={
            input_name: dynamic_axes_input,
            "note_onsets": {0: "batch_size"},
            "note_offsets": {0: "batch_size"},
            "note_frames": {0: "batch_size"},
            "note_velocity": {0: "batch_size"},
            "pedal_onset": {0: "batch_size"},
            "pedal_offset": {0: "batch_size"},
            "pedal_frame": {0: "batch_size"},
        },
    )
    print("    [OK] FP32 ONNX export complete.")

    if quantize:
        if HAS_ONNX_QUANT:
            int8_onnx_path = os.path.join(output_dir, f"{prefix}_int8.onnx")
            print(f"[*] Quantizing ONNX model to dynamic INT8: {int8_onnx_path}...")
            # Note: Do not quantize 'Conv' dynamically as it creates mixed U8/I8 types
            # that break Tract-ONNX inference.
            quantize_dynamic(
                model_input=fp32_onnx_path,
                model_output=int8_onnx_path,
                weight_type=QuantType.QInt8,
                op_types_to_quantize=["MatMul", "Gemm"],
            )
            print("    [OK] INT8 Dynamic ONNX quantization complete.")
            print("    [!] Note: Tract-ONNX in MeliorSonus uses FP32 ONNX by default.")
        else:
            print("[!] onnxruntime not installed, skipping ONNX quantization.")


def export_litert(model, output_dir, acoustic_only=True):
    if not HAS_AI_EDGE_TORCH:
        print("\n[!] LiteRT / TFLite export via ai-edge-torch is not available in this environment.")
        if IS_WINDOWS and AI_EDGE_TORCH_STATUS == "missing_torch_xla":
            print("    Reason: Google's 'ai-edge-torch' requires 'torch_xla' (StableHLO/OpenXLA),")
            print("    which does not have native Windows wheels or MSVC build support.")
            print("    -> Solution: Run the export inside WSL2 (Windows Subsystem for Linux):")
            print("         python export_bytedance_crnn.py --wsl")
            print("       Or execute 'run_wsl_export.bat'.")
        elif AI_EDGE_TORCH_STATUS == "missing_tensorflow":
            print("    Reason: 'tensorflow' is required by ai-edge-torch. Install via: pip install tensorflow")
        else:
            print(f"    Reason: {AI_EDGE_TORCH_ERR}")
        return False

    os.makedirs(output_dir, exist_ok=True)
    model.eval()

    prefix = "bytedance_crnn_acoustic" if acoustic_only else "bytedance_crnn_end_to_end"
    if acoustic_only:
        sample_input = torch.zeros((1, 1, 32, 229), dtype=torch.float32)
    else:
        sample_input = torch.zeros((1, 16000), dtype=torch.float32)

    # 1. FP32 LiteRT Model
    fp32_tflite_path = os.path.join(output_dir, f"{prefix}_fp32.tflite")
    print(f"[*] Exporting FP32 LiteRT model to {fp32_tflite_path}...")
    try:
        edge_model = litert_module.convert(model, (sample_input,))
        edge_model.export(fp32_tflite_path)
        print("    [OK] FP32 LiteRT model exported.")
    except Exception as e:
        print(f"[!] FP32 LiteRT export failed: {e}")

    # 2. Dynamic Range Quantization (Weight-only INT8)
    drq_tflite_path = os.path.join(output_dir, f"{prefix}_drq_int8.tflite")
    print(f"[*] Exporting Dynamic Range INT8 LiteRT model to {drq_tflite_path}...")
    try:
        import tensorflow as tf
        edge_model_drq = litert_module.convert(
            model,
            (sample_input,),
            _ai_edge_converter_flags={"optimizations": [tf.lite.Optimize.DEFAULT]},
        )
        edge_model_drq.export(drq_tflite_path)
        print("    [OK] Dynamic Range INT8 LiteRT model exported.")
    except Exception as e:
        print(f"[!] Dynamic Range INT8 LiteRT export failed: {e}")

    # 3. Post-Training Full Integer Quantization (PT2E PTQ)
    if HAS_AI_EDGE_QUANT:
        ptq_tflite_path = os.path.join(output_dir, f"{prefix}_ptq_int8.tflite")
        print(f"[*] Exporting PT2E Full Integer Quantized model to {ptq_tflite_path}...")
        try:
            exported_model = torch.export.export(model, (sample_input,))
            quantizer = PT2EQuantizer().set_global(get_symmetric_quantization_config())
            prepared = prepare_pt2e(exported_model, quantizer)

            print("    -> Running calibration over 50 frames...")
            with torch.no_grad():
                for _ in range(50):
                    dummy_chunk = torch.randn_like(sample_input) * 0.15
                    prepared(dummy_chunk)

            quantized = convert_pt2e(prepared)
            edge_model_ptq = litert_module.convert(quantized, (sample_input,))
            edge_model_ptq.export(ptq_tflite_path)
            print("    [OK] PT2E Full Integer Quantized LiteRT model exported.")
        except Exception as e:
            print(f"[!] PT2E Quantization encountered an error: {e}")

    return True


# =====================================================================
# 4. WSL2 Automated Runner
# =====================================================================
def run_via_wsl():
    """
    Executes this export script inside WSL2 using the installed Linux environment.
    """
    import subprocess
    print("=================================================================")
    print(" MeliorSonus Neural Transcriber: Running via WSL2 Bridge        ")
    print("=================================================================")
    try:
        distro_output = subprocess.check_output(["wsl.exe", "-l", "-q"], text=True)
        print(f"[*] Detected WSL distribution(s):\n{distro_output.strip()}")
    except Exception as e:
        print(f"[!] WSL2 check failed: {e}")
        return False

    script_path = os.path.abspath(__file__)
    drive = script_path[0].lower()
    wsl_path = f"/mnt/{drive}" + script_path[2:].replace("\\", "/")
    wsl_dir = os.path.dirname(wsl_path)

    cli_args = [arg for arg in sys.argv[1:] if arg != "--wsl"]
    cli_str = " ".join(f'"{a}"' for a in cli_args)

    wsl_script = f"{wsl_dir}/export_wsl.sh"
    cmd = f'cd "{wsl_dir}" && bash "{wsl_script}" {cli_str}'
    print(f"[*] Executing inside WSL2: {cmd}\n")
    ret = subprocess.call(["wsl.exe", "bash", "-c", cmd])
    return ret == 0


# =====================================================================
# 5. Main Entry Point
# =====================================================================
def main():
    parser = argparse.ArgumentParser(description="Export ByteDance CRNN to ONNX and TFLite")
    parser.add_argument("--output-dir", type=str, default="./models", help="Output directory for model weights")
    parser.add_argument("--checkpoint", type=str, default=None, help="Path to pretrained PyTorch weights (.pt/.pth)")
    parser.add_argument("--unidirectional", action="store_true", help="Use Unidirectional GRU for zero-lookahead streaming")
    parser.add_argument("--end-to-end", action="store_true", help="Export with in-graph MelSpectrogram (default: False, acoustic-only)")
    parser.add_argument("--sequence", action="store_true", help="Export full sequence outputs [batch, time, 88] instead of latest frame")
    parser.add_argument("--quantize-onnx", action="store_true", help="Also generate dynamic INT8 ONNX model for ONNX Runtime")
    parser.add_argument("--wsl", action="store_true", help="Execute export inside WSL2 (required on Windows for Google ai-edge-torch)")
    args = parser.parse_args()

    if args.wsl:
        success = run_via_wsl()
        sys.exit(0 if success else 1)

    print("=================================================================")
    print(" MeliorSonus Neural Audio: ByteDance CRNN Export & Quantization  ")
    print("=================================================================")

    crnn_core = ByteDanceCRNNAcoustic(
        bidirectional=not args.unidirectional,
        sequence_mode=args.sequence,
    )
    if args.checkpoint and os.path.exists(args.checkpoint):
        load_checkpoint(crnn_core, args.checkpoint)
    else:
        print("[!] No checkpoint provided; using initialized architecture for export scaffolding.")

    acoustic_only = not args.end_to_end
    if acoustic_only:
        print("[*] Exporting Acoustic-Only Model (Log-Mel [1, 1, T, 229] -> 7 Heads)")
        export_model = crnn_core
    else:
        print("[*] Exporting End-to-End Model (Raw Audio PCM [1, N] -> 7 Heads)")
        export_model = MeliorSonusCRNNEndToEnd(crnn_core)

    export_model.eval()
    export_onnx(
        export_model,
        args.output_dir,
        acoustic_only=acoustic_only,
        quantize=args.quantize_onnx,
    )
    export_litert(export_model, args.output_dir, acoustic_only=acoustic_only)

    print("\n[OK] ByteDance CRNN 7-Tensor Export Pipeline Finished!")


if __name__ == "__main__":
    main()
