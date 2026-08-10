# Workflow: KMP Platform Specifics
activation: glob
files: shared/src/**/*

This project supports multiple platforms. Use these commands to target specific platform builds within the shared module.

## Shared Module Targets

- **Android:**
  `./gradlew :shared:assembleDebug`
- **JVM (Desktop):**
  `./gradlew :shared:jvmJar`
- **JS (Web):**
  `./gradlew :shared:jsBrowserDevelopmentLibrary`
- **iOS:**
  `./gradlew :shared:iosArm64Main` (and other ios variants)

## Application Execution

- **Desktop (JVM):**
  `./gradlew :desktopApp:run`
- **Android:**
  `./gradlew :androidApp:installDebug`
- **Web (JS):**
  `./gradlew :webApp:jsBrowserDevelopmentRun`

## Platform-Specific Source Sets

When modifying code, ensure you are in the correct source set:
- `commonMain`: Shared logic (Decompose, Koin, SQLDelight interfaces).
- `androidMain`: Android-specific implementations (Drivers, etc.).
- `jvmMain`: Desktop-specific implementations.
- `jsMain`: Web-specific implementations.
- `iosMain`: iOS-specific implementations.
- `appleMain`: Shared logic for all Apple platforms (iOS, macOS, etc.) if configured.
