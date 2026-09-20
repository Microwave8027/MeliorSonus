# MeliorSonus Rust Workflows & Cargo Execution Guide

This document outlines how to properly execute Rust and Cargo tasks directly from the project root in this Kotlin Multiplatform (KMP) workspace.

---

## 1. Project Structure & Manifest Location

In MeliorSonus, the Rust codebase resides under `shared/` with the crate manifest located at:
```
shared/Cargo.toml
```

- **Rust Source Code**: `shared/src/commonMain/rust/`
- **Rust Examples / CLI Tests**: `shared/rust_examples/`
- **UniFFI Bindings**: Configured in `shared/build.gradle.kts`

---

## 2. Direct Cargo Commands from the Root Directory

Always pass `--manifest-path shared/Cargo.toml` when executing Cargo from the workspace root.

### 🧪 Running Unit Tests
```bash
# Run all unit tests
cargo test --manifest-path shared/Cargo.toml

# Run all unit tests with live console output
cargo test --manifest-path shared/Cargo.toml -- --nocapture

# Run a specific unit test by name
cargo test --manifest-path shared/Cargo.toml -- test_note_lifecycle_attack_decay_release
cargo test --manifest-path shared/Cargo.toml -- test_instrument_dynamic_mappings_staccato_and_tenuto
cargo test --manifest-path shared/Cargo.toml -- test_mashed_key_detection
```

### 🔍 Code Quality & Linter (Clippy)
```bash
# Run compiler check
cargo check --manifest-path shared/Cargo.toml

# Run strict Clippy across library, tests, and examples
cargo clippy --manifest-path shared/Cargo.toml --all-targets
```

### 🎙️ Running Examples & CLI Tools
```bash
# Run the real-time microphone pitch & articulation listener
cargo run --manifest-path shared/Cargo.toml --example mic_listener

# Check that all examples compile without running
cargo check --manifest-path shared/Cargo.toml --examples
```

### 📦 Building Release Binaries / Libraries
```bash
# Build release libraries (cdylib, staticlib, rlib)
cargo build --manifest-path shared/Cargo.toml --release
```

---

## 3. Gradle-Integrated Cargo Tasks (UniFFI & Cross-Compilation)

Gradle manages UniFFI generation and native compilation for multiplatform targets (Android NDK, iOS, Desktop).

### Check & Build Rust for Android/Multiplatform Targets
```bash
# Check all Rust compilation targets configured in Gradle
./gradlew :shared:cargoCheck

# Build all native Rust binaries for active target architectures
./gradlew :shared:cargoBuild

# Target-specific checks
./gradlew :shared:cargoCheckAndroidArmV7Debug
./gradlew :shared:cargoCheckAndroidX86Debug

# Full multiplatform build verification
./gradlew check -x test
```

---

## 4. Developer Productivity Shortcuts

To avoid typing `--manifest-path shared/Cargo.toml` every time, you can add shell aliases or functions.

### PowerShell (Add to `$PROFILE`)
```powershell
function ctest { cargo test --manifest-path shared/Cargo.toml @args }
function ccheck { cargo check --manifest-path shared/Cargo.toml @args }
function cclippy { cargo clippy --manifest-path shared/Cargo.toml --all-targets @args }
function cmic { cargo run --manifest-path shared/Cargo.toml --example mic_listener @args }
```

### Bash / Zsh (Add to `~/.bashrc` or `~/.zshrc`)
```bash
alias ctest='cargo test --manifest-path shared/Cargo.toml'
alias ccheck='cargo check --manifest-path shared/Cargo.toml'
alias cclippy='cargo clippy --manifest-path shared/Cargo.toml --all-targets'
alias cmic='cargo run --manifest-path shared/Cargo.toml --example mic_listener'
```

---

## 5. Summary Cheat Sheet

| Action | Root Command |
|---|---|
| **Run All Tests** | `cargo test --manifest-path shared/Cargo.toml` |
| **Run Tests (Verbose)** | `cargo test --manifest-path shared/Cargo.toml -- --nocapture` |
| **Lint & Clippy** | `cargo clippy --manifest-path shared/Cargo.toml --all-targets` |
| **Run Mic Listener** | `cargo run --manifest-path shared/Cargo.toml --example mic_listener` |
| **Gradle Multiplatform Verification** | `./gradlew check -x test` |
| **Gradle Rust Target Build** | `./gradlew :shared:cargoBuild` |
