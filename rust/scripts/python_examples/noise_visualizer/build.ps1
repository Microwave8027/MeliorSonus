<#
.SYNOPSIS
    Builds the Rust audio visualizer library and generates Python UniFFI bindings.

.DESCRIPTION
    Compiles the 'audio_visualizer' cdylib example from the shared MeliorSonus Rust crate,
    copies the resulting audio_visualizer.dll into the python script directory, and generates
    the UniFFI Python bindings (compose_app.py) using the crate's uniffi-bindgen binary.

.PARAMETER Release
    Build in release mode with optimizations. Defaults to debug mode.

.PARAMETER SkipBindings
    Skip generating UniFFI Python bindings (only compile and copy audio_visualizer.dll).

.PARAMETER SyncEnv
    Run 'uv sync' in the python script directory to ensure virtual environment dependencies are up to date.

.PARAMETER Run
    Immediately run main.py after a successful build.

.PARAMETER WavPath
    Optional path to a .wav audio file to pass to main.py (only used with -Run).

.EXAMPLE
    .\build.ps1
    Builds audio_visualizer in debug mode, copies DLL, and generates Python bindings.

.EXAMPLE
    .\build.ps1 -Release
    Builds in release mode with optimizations.

.EXAMPLE
    .\build.ps1 -Run
    Builds and immediately executes the visualizer via main.py.
#>

[CmdletBinding()]
param(
    [switch]$Release,
    [switch]$SkipBindings,
    [switch]$SyncEnv,
    [switch]$Run,
    [string]$WavPath
)

$ErrorActionPreference = "Stop"

# Determine directory paths
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { (Get-Location).Path }
$SharedDir = [System.IO.Path]::GetFullPath((Join-Path $ScriptDir "..\..\.."))
$CargoToml = Join-Path $SharedDir "Cargo.toml"

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host " MeliorSonus Noise Visualizer Build Script" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "Script directory : $ScriptDir"
Write-Host "Shared crate dir : $SharedDir"

# Validate requirements
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo was not found in PATH. Please install Rust or add cargo to your environment PATH."
    exit 1
}

if (-not (Test-Path $CargoToml)) {
    Write-Error "Could not find Cargo.toml at: $CargoToml"
    exit 1
}

$Profile = if ($Release) { "release" } else { "debug" }
Write-Host "Build profile    : $Profile"
Write-Host ""

# ---------------------------------------------------------------------------
# Step 1: Compile Rust cdylib example
# ---------------------------------------------------------------------------
Write-Host "[1/3] Compiling Rust example 'audio_visualizer' ($Profile)..." -ForegroundColor Yellow
$CargoBuildArgs = @("build", "--example", "audio_visualizer")
if ($Release) {
    $CargoBuildArgs += "--release"
}

Push-Location $SharedDir
try {
    & cargo @CargoBuildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed with exit code $LASTEXITCODE"
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}

# Locate built artifacts
$TargetDir = Join-Path $SharedDir "target\$Profile\examples"
$BuiltDll = Join-Path $TargetDir "audio_visualizer.dll"
$BuiltPdb = Join-Path $TargetDir "audio_visualizer.pdb"
$DestDll = Join-Path $ScriptDir "audio_visualizer.dll"
$DestPdb = Join-Path $ScriptDir "audio_visualizer.pdb"

if (-not (Test-Path $BuiltDll)) {
    Write-Error "Expected build artifact not found at: $BuiltDll"
    exit 1
}

# ---------------------------------------------------------------------------
# Step 2: Copy DLL (and PDB if present) to python script directory
# ---------------------------------------------------------------------------
Write-Host "[2/3] Copying DLL to visualizer directory..." -ForegroundColor Yellow
Copy-Item -Path $BuiltDll -Destination $DestDll -Force
Write-Host "  -> Copied: $DestDll" -ForegroundColor Green

if (Test-Path $BuiltPdb) {
    Copy-Item -Path $BuiltPdb -Destination $DestPdb -Force
    Write-Host "  -> Copied: $DestPdb" -ForegroundColor Green
}

# ---------------------------------------------------------------------------
# Step 3: Generate UniFFI Python Bindings
# ---------------------------------------------------------------------------
if (-not $SkipBindings) {
    Write-Host "[3/3] Generating UniFFI Python bindings (compose_app.py)..." -ForegroundColor Yellow
    $BindgenArgs = @(
        "run",
        "--bin", "uniffi-bindgen",
        "--",
        "generate",
        $BuiltDll,
        "--library",
        "--language", "python",
        "--out-dir", $ScriptDir,
        "--no-format"
    )

    Push-Location $SharedDir
    try {
        & cargo @BindgenArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Error "UniFFI bindings generation failed with exit code $LASTEXITCODE"
            exit $LASTEXITCODE
        }
    }
    finally {
        Pop-Location
    }

    $GeneratedPy = Join-Path $ScriptDir "compose_app.py"
    if (Test-Path $GeneratedPy) {
        Write-Host "  -> Updated bindings: $GeneratedPy" -ForegroundColor Green
    }
} else {
    Write-Host "[3/3] Skipping UniFFI bindings generation (-SkipBindings requested)" -ForegroundColor DarkGray
}

# ---------------------------------------------------------------------------
# Optional: Environment sync with uv
# ---------------------------------------------------------------------------
if ($SyncEnv) {
    Write-Host ""
    Write-Host "Syncing Python virtual environment..." -ForegroundColor Yellow
    if (Get-Command uv -ErrorAction SilentlyContinue) {
        Push-Location $ScriptDir
        try {
            & uv sync
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "uv sync finished with exit code $LASTEXITCODE"
            } else {
                Write-Host "  -> Python environment synced successfully." -ForegroundColor Green
            }
        }
        finally {
            Pop-Location
        }
    } else {
        Write-Warning "'uv' command not found in PATH. Skipping environment sync."
    }
}

Write-Host ""
Write-Host "========================================================" -ForegroundColor Green
Write-Host " Build completed successfully!" -ForegroundColor Green
Write-Host "========================================================" -ForegroundColor Green

# ---------------------------------------------------------------------------
# Optional: Run visualizer
# ---------------------------------------------------------------------------
if ($Run) {
    Write-Host ""
    Write-Host "Starting noise visualizer..." -ForegroundColor Cyan
    Push-Location $ScriptDir
    try {
        $PyArgs = @("main.py")
        if ($WavPath) {
            $PyArgs += $WavPath
        }

        if (Get-Command uv -ErrorAction SilentlyContinue) {
            & uv run python @PyArgs
        } else {
            & python @PyArgs
        }
    }
    finally {
        Pop-Location
    }
}
