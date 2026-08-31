#!/usr/bin/env bash
# =============================================================================
# build_ios_bazel.sh
#
# Builds libtensorflowlite_c.a (static C API library) from the LiteRT source
# for iOS ARM64 device and ARM64 simulator targets using Bazel.
#
# Prerequisites:
#   1. macOS with Xcode 15+ and Command Line Tools installed.
#   2. Bazel 7.x+ (check .bazelversion in the LiteRT repo for exact version).
#      Install via Bazelisk: `brew install bazelisk`
#   3. Git (to clone the LiteRT repo if not already present).
#   4. Python 3.x on PATH (required by TFLite's configure script).
#
# Usage:
#   ./build_ios_bazel.sh [--litert-src /path/to/LiteRT]
#
# Output:
#   ../../shared/native_libs/litert/ios-arm64/libtensorflowlite_c.a
#   ../../shared/native_libs/litert/ios-arm64-sim/libtensorflowlite_c.a
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_BASE="$PROJECT_ROOT/shared/native_libs/litert"

# Check we're on macOS
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ERROR: This script must be run on macOS (required for Xcode / iOS SDK)."
  exit 1
fi

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
LITERT_SRC=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --litert-src) LITERT_SRC="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ---------------------------------------------------------------------------
# Clone or locate LiteRT source
# ---------------------------------------------------------------------------
if [[ -z "$LITERT_SRC" ]]; then
  LITERT_SRC="$SCRIPT_DIR/_litert_src"
  if [[ ! -d "$LITERT_SRC" ]]; then
    echo "=== Cloning google-ai-edge/LiteRT …"
    git clone --depth 1 https://github.com/google-ai-edge/LiteRT.git "$LITERT_SRC"
  else
    echo "=== Using cached LiteRT source at $LITERT_SRC"
  fi
fi

echo "=== LiteRT source: $LITERT_SRC"

# ---------------------------------------------------------------------------
# Detect Bazel target path
#
# The C API Bazel target has moved across LiteRT versions:
#   Legacy:  //tensorflow/lite/c:tensorflowlite_c
#   Current: //tflite/c:tensorflowlite_c
# We try the current path first and fall back to legacy.
# ---------------------------------------------------------------------------
BAZEL_TARGET_NEW="//tflite/c:tensorflowlite_c"
BAZEL_TARGET_LEGACY="//tensorflow/lite/c:tensorflowlite_c"

if [[ -d "$LITERT_SRC/tflite/c" ]]; then
  BAZEL_TARGET="$BAZEL_TARGET_NEW"
elif [[ -d "$LITERT_SRC/tensorflow/lite/c" ]]; then
  BAZEL_TARGET="$BAZEL_TARGET_LEGACY"
else
  echo "ERROR: Cannot locate the C API directory in $LITERT_SRC"
  exit 1
fi

echo "=== Bazel target: $BAZEL_TARGET"

# ---------------------------------------------------------------------------
# Configure (if needed — TFLite's configure sets up Python, iOS support etc.)
# ---------------------------------------------------------------------------
pushd "$LITERT_SRC" > /dev/null

if [[ -f "configure" ]]; then
  echo "=== Running configure script (selecting iOS support) …"
  # Auto-accept defaults; enable iOS. The script reads env vars.
  export TF_CONFIGURE_IOS=1
  yes "" 2>/dev/null | python3 configure 2>/dev/null || true
fi

# ---------------------------------------------------------------------------
# Build function — called once per platform config
# ---------------------------------------------------------------------------
build_for_platform() {
  local BAZEL_CONFIG="$1"  # e.g. ios_arm64
  local OUT_DIR_NAME="$2"  # e.g. ios-arm64
  local MIN_IOS_VERSION="${3:-15.0}"

  echo ""
  echo "====================================================================="
  echo "  Building libtensorflowlite_c.a (Bazel: --config=$BAZEL_CONFIG)"
  echo "  Minimum iOS version: $MIN_IOS_VERSION"
  echo "====================================================================="

  # Build a static framework / static library.
  # -c opt enables optimisations.
  # --ios_minimum_os sets deployment target.
  bazel build \
    --config="$BAZEL_CONFIG" \
    -c opt \
    --ios_minimum_os="$MIN_IOS_VERSION" \
    --define tflite_with_xnnpack=true \
    --copt=-fembed-bitcode \
    "$BAZEL_TARGET"

  # Locate the output .a inside Bazel's output tree
  local BAZEL_BIN
  BAZEL_BIN="$(bazel info --config="$BAZEL_CONFIG" bazel-bin)"

  local LIB_FILE=""
  # Try common output locations
  for CANDIDATE in \
    "$BAZEL_BIN/tflite/c/libtensorflowlite_c.a" \
    "$BAZEL_BIN/tensorflow/lite/c/libtensorflowlite_c.a" \
    "$BAZEL_BIN/tflite/c/libtflite_c.a" \
  ; do
    if [[ -f "$CANDIDATE" ]]; then
      LIB_FILE="$CANDIDATE"
      break
    fi
  done

  if [[ -z "$LIB_FILE" ]]; then
    echo "WARNING: Expected .a not found, searching Bazel output …"
    LIB_FILE="$(find "$BAZEL_BIN" -name 'libtensorflowlite_c.a' -print -quit 2>/dev/null || true)"
  fi

  if [[ -z "$LIB_FILE" ]]; then
    echo "ERROR: Could not find libtensorflowlite_c.a in $BAZEL_BIN"
    echo "  Available .a files:"
    find "$BAZEL_BIN" -name '*.a' | head -20
    exit 1
  fi

  local DEST_DIR="$OUTPUT_BASE/$OUT_DIR_NAME"
  mkdir -p "$DEST_DIR"
  cp -v "$LIB_FILE" "$DEST_DIR/libtensorflowlite_c.a"

  echo "=== ✅  $DEST_DIR/libtensorflowlite_c.a"
}

# ---------------------------------------------------------------------------
# Build iOS device (ARM64) and simulator (ARM64)
# ---------------------------------------------------------------------------
build_for_platform "ios_arm64"     "ios-arm64"     "15.0"
build_for_platform "ios_sim_arm64" "ios-arm64-sim" "15.0"

popd > /dev/null

echo ""
echo "====================================================================="
echo "  All iOS builds complete."
echo "  Static libraries are in:"
echo "    $OUTPUT_BASE/ios-arm64/libtensorflowlite_c.a"
echo "    $OUTPUT_BASE/ios-arm64-sim/libtensorflowlite_c.a"
echo ""
echo "  Remember to link the following frameworks in build.rs for iOS:"
echo "    - Accelerate.framework"
echo "    - CoreML.framework"
echo "    - libc++ (stdc++)"
echo "====================================================================="
