#!/usr/bin/env bash
# =============================================================================
# build_android_cmake.sh
#
# Builds libtensorflowlite_c.a (static C API library) from the LiteRT source
# for Android ARM64 and ARM32 targets using CMake + the Android NDK toolchain.
#
# Prerequisites:
#   1. Android NDK r26+ installed (set ANDROID_NDK_HOME or pass --ndk-path).
#   2. CMake 3.16+ on PATH.
#   3. Git (to clone the LiteRT repo if not already present).
#   4. Ninja (optional but recommended for faster builds).
#
# Usage:
#   ./build_android_cmake.sh [--ndk-path /path/to/ndk] [--litert-src /path/to/LiteRT]
#
# Output:
#   ../../shared/native_libs/litert/android-arm64/libtensorflowlite_c.a
#   ../../shared/native_libs/litert/android-arm32/libtensorflowlite_c.a
# =============================================================================
set -euo pipefail

# Enable long file paths on Windows for Git checkouts
git config --global core.longpaths true 2>/dev/null || true

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_BASE="$PROJECT_ROOT/shared/native_libs/litert"

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
NDK_PATH="${ANDROID_NDK_HOME:-}"
LITERT_SRC=""
CMAKE_BIN=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --ndk-path)    NDK_PATH="$2"; shift 2 ;;
    --litert-src)  LITERT_SRC="$2"; shift 2 ;;
    --cmake-path)       CMAKE_BIN="$2"; shift 2 ;;
    --host-tools-dir)   HOST_TOOLS_DIR="$2"; shift 2 ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

if [[ -z "$NDK_PATH" ]]; then
  echo "ERROR: Android NDK not found."
  echo "  Set ANDROID_NDK_HOME or pass --ndk-path /path/to/android-ndk-r26d"
  exit 1
fi

# Convert Windows path (e.g. C:\foo or C:/foo) to POSIX path (/c/foo in Git Bash, /mnt/c/foo in WSL)
to_posix_path() {
  local p="$1"
  if [[ "$p" =~ ^/ ]]; then
    echo "$p"
    return
  fi
  if command -v wslpath &> /dev/null; then
    wslpath -u "$p" 2>/dev/null || echo "$p"
  elif command -v cygpath &> /dev/null; then
    cygpath -u "$p"
  elif [[ "$p" =~ ^([a-zA-Z]):[/\\](.*) ]]; then
    local drive="${BASH_REMATCH[1]}"
    drive=$(echo "$drive" | tr '[:upper:]' '[:lower:]')
    local rest="${BASH_REMATCH[2]//\\//}"
    if [[ -d "/mnt/$drive" ]]; then
      echo "/mnt/$drive/$rest"
    else
      echo "/$drive/$rest"
    fi
  else
    echo "${p//\\//}"
  fi
}

# Convert POSIX path back to Windows path format if cmake is a Windows executable (.exe)
to_cmake_path() {
  local p="$1"
  if [[ "$CMAKE_CMD" == *.exe ]] || [[ "$CMAKE_CMD" == *bin/cmake.exe ]]; then
    if command -v wslpath &> /dev/null; then
      wslpath -w "$p" 2>/dev/null | tr '\\' '/' || echo "$p"
    elif command -v cygpath &> /dev/null; then
      cygpath -w "$p" | tr '\\' '/'
    elif [[ "$p" =~ ^/mnt/([a-zA-Z])/(.*) || "$p" =~ ^/([a-zA-Z])/(.*) ]]; then
      local drive="${BASH_REMATCH[1]}"
      drive=$(echo "$drive" | tr '[:lower:]' '[:upper:]')
      local rest="${BASH_REMATCH[2]}"
      echo "$drive:/$rest"
    else
      echo "$p"
    fi
  else
    echo "$p"
  fi
}

NDK_PATH="$(to_posix_path "$NDK_PATH")"
echo "=== Android NDK: $NDK_PATH"

# ---------------------------------------------------------------------------
# Locate CMake & Ninja (Check PATH first, then Android SDK cmake/ folder)
# ---------------------------------------------------------------------------
CMAKE_CMD="cmake"

if [[ -n "$CMAKE_BIN" ]]; then
  CMAKE_CMD="$(to_posix_path "$CMAKE_BIN")"
elif command -v cmake &> /dev/null; then
  CMAKE_CMD="cmake"
else
  # Try to find CMake inside Android SDK
  SDK_DIR="${NDK_PATH%/ndk/*}"
  POSSIBLE_CMAKE=""
  for candidate in \
    $SDK_DIR/cmake/*/bin/cmake.exe \
    $SDK_DIR/cmake/*/bin/cmake \
    "$SDK_DIR/cmake/4.1.2/bin/cmake.exe" \
    "$SDK_DIR/cmake/3.22.1/bin/cmake.exe" \
    "/c/Users/micro/AppData/Local/Android/Sdk/cmake/4.1.2/bin/cmake.exe" \
    "/c/Program Files/CMake/bin/cmake.exe" \
  ; do
    if [[ -f "$candidate" ]]; then
      POSSIBLE_CMAKE="$candidate"
      break
    fi
  done

  if [[ -n "$POSSIBLE_CMAKE" ]]; then
    CMAKE_CMD="$(to_posix_path "$POSSIBLE_CMAKE")"
    CMAKE_DIR="$(dirname "$CMAKE_CMD")"
    echo "=== Auto-detected CMake in Android SDK: $CMAKE_DIR"
    export PATH="$CMAKE_DIR:$PATH"
  else
    echo "ERROR: cmake command not found."
    echo "  Options to fix:"
    echo "  1. Pass --cmake-path \"C:/path/to/cmake.exe\""
    echo "  2. Install CMake in Android Studio: Settings -> SDK Tools -> CMake"
    echo "  3. Install via terminal: winget install Kitware.CMake Ninja-build.Ninja"
    exit 1
  fi
fi

echo "=== CMake version: $("$CMAKE_CMD" --version | head -n 1)"

# Check if Ninja is available
CMAKE_GEN_FLAG=""
NINJA_BIN=""
if [[ -f "$(dirname "$CMAKE_CMD")/ninja.exe" ]]; then
  NINJA_BIN="$(dirname "$CMAKE_CMD")/ninja.exe"
elif [[ -f "$(dirname "$CMAKE_CMD")/ninja" ]]; then
  NINJA_BIN="$(dirname "$CMAKE_CMD")/ninja"
elif command -v ninja &> /dev/null; then
  NINJA_BIN="$(command -v ninja)"
fi

if [[ -n "$NINJA_BIN" ]]; then
  echo "=== Ninja detected: $NINJA_BIN"
  CMAKE_GEN_FLAG="-GNinja -DCMAKE_MAKE_PROGRAM=$(to_cmake_path "$NINJA_BIN")"
  export PATH="$(dirname "$NINJA_BIN"):$PATH"
  export WSLENV="PATH/l:${WSLENV:-}"
fi

# Locate Git executable for CMake ExternalProject/FetchContent
GIT_BIN=""
if [[ -f "/mnt/c/Program Files/Git/cmd/git.exe" ]]; then
  GIT_BIN="/mnt/c/Program Files/Git/cmd/git.exe"
elif [[ -f "/c/Program Files/Git/cmd/git.exe" ]]; then
  GIT_BIN="/c/Program Files/Git/cmd/git.exe"
elif command -v git &> /dev/null; then
  GIT_BIN="$(command -v git)"
fi

GIT_FLAG=""
if [[ -n "$GIT_BIN" ]]; then
  GIT_FLAG="-DGIT_EXECUTABLE=$(to_cmake_path "$GIT_BIN")"
  export PATH="$(dirname "$GIT_BIN"):$PATH"
fi

# ---------------------------------------------------------------------------
# Ensure Host flatc is available for cross-compilation
# ---------------------------------------------------------------------------
HOST_TOOLS_DIR="${HOST_TOOLS_DIR:-}"

if [[ -z "$HOST_TOOLS_DIR" ]]; then
  HOST_TOOLS_DIR="$SCRIPT_DIR/_build_host_tools"
fi
HOST_TOOLS_DIR="$(to_posix_path "$HOST_TOOLS_DIR")"

if command -v flatc &> /dev/null; then
  FLATC_BIN="$(command -v flatc)"
  HOST_TOOLS_DIR="$(dirname "$FLATC_BIN")"
  echo "=== Host flatc found on PATH: $FLATC_BIN"
elif [[ -f "$HOST_TOOLS_DIR/bin/flatc.exe" ]] || [[ -f "$HOST_TOOLS_DIR/bin/flatc" ]] || [[ -f "$HOST_TOOLS_DIR/flatc.exe" ]] || [[ -f "$HOST_TOOLS_DIR/flatc" ]]; then
  echo "=== Using cached host flatc in $HOST_TOOLS_DIR"
else
  echo "=== Building host flatc compiler (required for cross-compilation) …"
  FLATBUFFERS_SRC="$SCRIPT_DIR/_flatbuffers_src"
  if [[ ! -d "$FLATBUFFERS_SRC" ]]; then
    git clone --depth 1 --branch v25.9.23 https://github.com/google/flatbuffers.git "$FLATBUFFERS_SRC"
  fi

  mkdir -p "$HOST_TOOLS_DIR/build"
  "$CMAKE_CMD" -S "$(to_cmake_path "$FLATBUFFERS_SRC")" -B "$(to_cmake_path "$HOST_TOOLS_DIR/build")" \
    -DFLATBUFFERS_BUILD_TESTS=OFF \
    -DFLATBUFFERS_BUILD_FLATLIB=OFF \
    -DFLATBUFFERS_BUILD_FLATHASH=OFF \
    -DCMAKE_BUILD_TYPE=Release \
    ${CMAKE_GEN_FLAG:-}

  "$CMAKE_CMD" --build "$(to_cmake_path "$HOST_TOOLS_DIR/build")" --target flatc --config Release

  mkdir -p "$HOST_TOOLS_DIR/bin"
  FLATC_FOUND="$(find "$HOST_TOOLS_DIR/build" -type f \( -name "flatc" -o -name "flatc.exe" \) 2>/dev/null | head -n 1 || true)"
  if [[ -n "$FLATC_FOUND" ]]; then
    cp "$FLATC_FOUND" "$HOST_TOOLS_DIR/bin/"
    echo "=== ✅ Host flatc built successfully in $HOST_TOOLS_DIR/bin"
  else
    echo "ERROR: Failed to find built flatc in $HOST_TOOLS_DIR/build"
    exit 1
  fi
fi

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

TFLITE_SRC="$LITERT_SRC/tflite"
if [[ ! -d "$TFLITE_SRC" ]]; then
  # Older repo layout: tensorflow/lite
  TFLITE_SRC="$LITERT_SRC/tensorflow/lite"
fi

if [[ ! -d "$TFLITE_SRC" ]]; then
  echo "ERROR: Cannot find tflite source directory in $LITERT_SRC"
  exit 1
fi

echo "=== TFLite source: $TFLITE_SRC"

# Ensure tflite/c/CMakeLists.txt uses correct TFLITE_SOURCE_DIR
if [[ -f "$LITERT_SRC/tflite/c/CMakeLists.txt" ]]; then
  if grep -q 'set(TFLITE_SOURCE_DIR "${TF_SOURCE_DIR}/tensorflow/lite")' "$LITERT_SRC/tflite/c/CMakeLists.txt" 2>/dev/null; then
    sed -i 's|set(TFLITE_SOURCE_DIR "${TF_SOURCE_DIR}/tensorflow/lite")|if(EXISTS "${TF_SOURCE_DIR}/tflite/CMakeLists.txt")\n  set(TFLITE_SOURCE_DIR "${TF_SOURCE_DIR}/tflite")\nelse()\n  set(TFLITE_SOURCE_DIR "${TF_SOURCE_DIR}/tensorflow/lite")\nendif()|g' "$LITERT_SRC/tflite/c/CMakeLists.txt" 2>/dev/null || true
  fi
fi

# ---------------------------------------------------------------------------
# Build function — called once per ABI
# ---------------------------------------------------------------------------
build_for_abi() {
  local ABI="$1"          # arm64-v8a | armeabi-v7a
  local OUT_DIR_NAME="$2" # android-arm64 | android-arm32
  local API_LEVEL="${3:-24}"

  echo ""
  echo "====================================================================="
  echo "  Building libtensorflowlite_c.a for $ABI (API $API_LEVEL)"
  echo "====================================================================="

  local BUILD_DIR="$SCRIPT_DIR/_build_android_${ABI}"
  mkdir -p "$BUILD_DIR"

  # Use tflite/c as the CMake project root if available (builds only C API + core, excludes proto tools)
  local CMAKE_ROOT="$TFLITE_SRC/c"
  if [[ ! -f "$CMAKE_ROOT/CMakeLists.txt" ]]; then
    CMAKE_ROOT="$TFLITE_SRC"
  fi

  if [[ -f "$BUILD_DIR/CMakeCache.txt" ]]; then
    if grep -q "CMAKE_PROJECT_NAME:STATIC=tensorflow-lite$" "$BUILD_DIR/CMakeCache.txt" 2>/dev/null; then
      echo "=== Resetting old CMake cache for clean C API build..."
      rm -rf "$BUILD_DIR/CMakeCache.txt" "$BUILD_DIR/CMakeFiles" "$BUILD_DIR/build.ninja"
    fi
  fi

  "$CMAKE_CMD" -S "$(to_cmake_path "$CMAKE_ROOT")" -B "$(to_cmake_path "$BUILD_DIR")" \
    -DCMAKE_TOOLCHAIN_FILE="$(to_cmake_path "$NDK_PATH/build/cmake/android.toolchain.cmake")" \
    -DANDROID_ABI="$ABI" \
    -DANDROID_NATIVE_API_LEVEL="$API_LEVEL" \
    -DANDROID_STL=c++_shared \
    -DCMAKE_BUILD_TYPE=Release \
    -DTFLITE_C_BUILD_SHARED_LIBS=OFF \
    -DTF_SOURCE_DIR="$(to_cmake_path "$LITERT_SRC")" \
    -DTFLITE_SOURCE_DIR="$(to_cmake_path "$TFLITE_SRC")" \
    -DTFLITE_ENABLE_XNNPACK=ON \
    -DTFLITE_ENABLE_GPU=ON \
    -DTFLITE_ENABLE_NNAPI=OFF \
    -DTFLITE_ENABLE_RESOURCE=OFF \
    -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DTFLITE_HOST_TOOLS_DIR="$(to_cmake_path "$HOST_TOOLS_DIR")" \
    ${GIT_BIN:+"-DGIT_EXECUTABLE=$(to_cmake_path "$GIT_BIN")"} \
    ${CMAKE_GEN_FLAG:-}

  # Build only the static C library target to avoid non-essential tools needing protoc
  if ! "$CMAKE_CMD" --build "$(to_cmake_path "$BUILD_DIR")" --target tensorflowlite_c --config Release -j "$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)"; then
    echo "=== Notice: tensorflowlite_c target failed or not found, trying tensorflow-lite..."
    "$CMAKE_CMD" --build "$(to_cmake_path "$BUILD_DIR")" --target tensorflow-lite --config Release -j "$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)"
  fi

  # Locate the output .a — CMake may place it in several locations
  local LIB_FILE
  LIB_FILE="$(find "$BUILD_DIR" -name 'libtensorflowlite_c.a' -print -quit 2>/dev/null || true)"

  if [[ -z "$LIB_FILE" ]]; then
    echo "WARNING: libtensorflowlite_c.a not found, trying libtensorflow-lite.a or libtflite_c.a …"
    LIB_FILE="$(find "$BUILD_DIR" \( -name 'libtensorflow-lite.a' -o -name 'libtflite_c.a' \) -print -quit 2>/dev/null || true)"
  fi

  if [[ -z "$LIB_FILE" ]]; then
    echo "ERROR: Could not find the static library in $BUILD_DIR"
    echo "  Available .a files:"
    find "$BUILD_DIR" -name '*.a' | head -20
    exit 1
  fi

  local DEST_DIR="$OUTPUT_BASE/$OUT_DIR_NAME"
  mkdir -p "$DEST_DIR"
  cp -v "$LIB_FILE" "$DEST_DIR/libtensorflowlite_c.a"

  echo "=== ✅  $DEST_DIR/libtensorflowlite_c.a"
}

# ---------------------------------------------------------------------------
# Build both architectures
# ---------------------------------------------------------------------------
build_for_abi "arm64-v8a"    "android-arm64" 24
build_for_abi "armeabi-v7a"  "android-arm32" 24

echo ""
echo "====================================================================="
echo "  All Android builds complete."
echo "  Static libraries are in:"
echo "    $OUTPUT_BASE/android-arm64/libtensorflowlite_c.a"
echo "    $OUTPUT_BASE/android-arm32/libtensorflowlite_c.a"
echo "====================================================================="
