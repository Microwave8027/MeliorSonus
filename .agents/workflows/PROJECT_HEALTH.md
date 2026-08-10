# Workflow: Project Health Verification
activation: glob
files: **/*

Use this workflow to verify that the project is in a good state after making changes, especially to the shared module or database schemas.

## Verification Sequence

1.  **Regenerate Generated Code:**
    Ensure SQLDelight and Wire classes are up to date.
    `./gradlew generateSqlDelightInterface generateProtos`

2.  **Build Shared Module:**
    Verify the common code compiles for all targets.
    `./gradlew :shared:assemble`

3.  **Run Unit Tests:**
    Run tests in the shared module to ensure no regressions.
    `./gradlew :shared:allTests`

4.  **Verify Platform Apps:**
    Ensure the main apps still build.
    - Android: `./gradlew :androidApp:assembleDebug`
    - Desktop: `./gradlew :desktopApp:assemble`

## Common Fixes

- **Unresolved SQLDelight references:**
  Run `./gradlew generateSqlDelightInterface`.
- **Unresolved Proto/Wire references:**
  Run `./gradlew generateProtos`.
- **Database Schema Mismatch:**
  Check `.sq` files and run `./gradlew verifySqlDelight` (if available).
- **Compose Multiplatform Resource issues:**
  Run `./gradlew generateComposeResClass`.
- **Build Cache Issues:**
  If strange compilation errors occur, try `./gradlew clean` then rebuild.
- **Dependency Issues:**
  Try `./gradlew --refresh-dependencies` or check `gradle/libs.versions.toml`.
