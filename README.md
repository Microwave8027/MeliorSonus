# MeliorSonus

> **Status: incomplete.** This project is a work in progress and is not yet usable end to end.

## The Idea

MeliorSonus is an app that helps users play music better. It renders an SVG of the sheet music from a MusicXML file and overlays practice advice directly on the score.

The audio pipeline is written in Rust. It analyzes what the user plays and compares it against references, such as the actual MusicXML, to measure how accurate the performance is. That feedback is what drives the advice shown on the rendered score.

It should also support comparing a performance against a YouTube video, showing how far off the user is from the reference.

## What's Implemented

- Real-time audio capture and WAV reading (`cpal`, `hound`), with resampling and filtering.
- DSP feature extraction: pitch detection (MPM/NSDF), onset detection, tempo tracking, spectral features, RMS/psychoacoustic loudness, and articulation/damping classification.
- Neural polyphonic note transcription (CRNN-based, run through `tract`/LiteRT) with a note segmenter.
- MusicXML/MXL score parsing and preprocessing, plus early score matching.
- A Kotlin Multiplatform app skeleton (Compose Multiplatform) with navigation and placeholder screens.

## Still To Do

- **UI:** a real user interface, including the SVG score rendering with advice overlays.
- **Comparison engine backend:** a good engine for comparing performances against the MusicXML (and other references) and scoring them.
- **LLM integration:** turning comparison results into practice advice.
- **Better CRNN:** a more accurate model for audio note detection.
- **YouTube comparison:** comparing against a YouTube video and measuring how far off the performance is.

## Project Structure

This is a Kotlin Multiplatform project targeting Android, iOS, Web, Desktop (JVM).

* [/iosApp](./iosApp/iosApp) contains an iOS application. Even if you’re sharing your UI with Compose Multiplatform,
  you need this entry point for your iOS app. This is also where you should add SwiftUI code for your project.

* [/shared](./shared/src) is for code that will be shared across your Compose Multiplatform applications.
  It contains several subfolders:
  - [commonMain](./shared/src/commonMain/kotlin) is for code that’s common for all targets.
  - Other folders are for Kotlin code that will be compiled for only the platform indicated in the folder name.
    For example, if you want to use Apple’s CoreCrypto for the iOS part of your Kotlin app,
    the [iosMain](./shared/src/iosMain/kotlin) folder would be the right place for such calls.
    Similarly, if you want to edit the Desktop (JVM) specific part, the [jvmMain](./shared/src/jvmMain/kotlin)
    folder is the appropriate location.

## Rust Workflows & DSP Engine

For instructions on running Cargo tests, Clippy linter, and real-time DSP microphone examples from the root, see [RUST_WORKFLOWS.md](./RUST_WORKFLOWS.md).

## Current Progress

Check out the TODO.md for more details on current progress. Rust is currently implemented via uniffi bindings but I have not written any. Will focus on writing everything inside of rust in order to avoid
ffi boundry issues. 