#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "================================================================="
echo " MeliorSonus CRNN Export: WSL2 / Linux Environment              "
echo "================================================================="

PY_LIB="$(find "$HOME/.local/share/uv/python" -name "libpython3.11.so.1.0" -printf '%h' -quit 2>/dev/null || true)"
if [ -n "$PY_LIB" ]; then
    export LD_LIBRARY_PATH="$PY_LIB:$LD_LIBRARY_PATH"
fi

export PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python

.venv-wsl/bin/python export_bytedance_crnn.py "$@"
