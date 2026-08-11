# Workflow: Gradle Build and Maintenance
activation: glob
files: **/*.gradle.kts, **/*.gradle, **/gradle.properties

This workflow provides essential Gradle commands for building, testing, and maintaining the MeliorSonus project.

## Build Commands

- **Build all modules:**
  `./gradlew build`
- **Assemble Android Debug APK:**
  `./gradlew :androidApp:assembleDebug`
- **Assemble Desktop App:**
  `./gradlew :desktopApp:assemble`
- **Assemble Web App (Production):**
  `./gradlew :webApp:jsBrowserDistribution`
- **Assemble Shared Module (Android):**
  `./gradlew :shared:assembleDebug`

## Run Commands

- **Run Desktop App:**
  `./gradlew :desktopApp:run`
- **Run Web App (Development):**
  `./gradlew :webApp:jsBrowserDevelopmentRun --continuous`
- **Run Android App:**
  Requires a device/emulator. Usually deployed via IDE, but:
  `./gradlew :androidApp:installDebug`

## Testing Commands

- **Run all tests:**
  `./gradlew test`
- **Run Shared Common Tests:**
  `./gradlew :shared:allTests`
- **Run Shared JVM Tests:**
  `./gradlew :shared:jvmTest`
- **Run Shared Android Tests:**
  `./gradlew :shared:testDebugUnitTest`
- **Run Android App Tests:**
  `./gradlew :androidApp:testDebugUnitTest`

## Code Generation

- **Generate SQLDelight code:**
  `./gradlew generateSqlDelightInterface`
- **Generate Wire (Proto) code:**
  `./gradlew generateProtos`
- **Clear and Regenerate everything:**
  `./gradlew clean generateSqlDelightInterface generateProtos`

## Maintenance and Debugging
- **Print full errors:**
  `./gradlew help --warning-mode all`
- **Build with Stacktrace:**
  `./gradlew build --stacktrace`
- **Clean Project:**
  `./gradlew clean`
- **Refresh Dependencies:**
  `./gradlew build --refresh-dependencies`
- **List Tasks:**
  `./gradlew tasks`
- **Check Dependency Updates:**
  `./gradlew dependencyUpdates` (if plugin is applied)
- **Analyze Dependencies:**
  `./gradlew :shared:dependencies`

## CI/CD and Verification

- **Full Verification (Lint + Test):**
  `./gradlew check`
- **Kotlin Linting (if applied):**
  `./gradlew ktlintCheck` or `./gradlew detekt`

## Cargo For Rust

- **checking for errors:**
  `cargo check --manifest-path shared/Cargo.toml`
- **build rust library:**
  `cargo build --manifest-path shared/Cargo.toml`
- **run rust tests:**
  `cargo test --manifest-path shared/Cargo.toml`
