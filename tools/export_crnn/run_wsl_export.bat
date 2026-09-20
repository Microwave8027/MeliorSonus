@echo off
echo =================================================================
echo  MeliorSonus ByteDance CRNN LiteRT/ONNX Export via WSL2
echo =================================================================

where wsl >nul 2>&1 || (
    echo [!] WSL not found. Please install WSL via: wsl --install
    exit /b 1
)

wsl bash -c "cd \"$(wslpath '%~dp0')\" && bash export_wsl.sh %*"
