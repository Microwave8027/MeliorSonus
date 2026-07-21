---
name: kmp-ios-framework-check
description: Reviews shared code interface parameters to prevent Kotlin/Native compiler crashes on iOS targets.
---
# Interop Standards
- **Generics:** Avoid using complex nested Kotlin collections (like `Map<String, List<CustomState>>`) directly in shared APIs accessed by Swift. Wrap them in clean, flat data models.
- **Coroutines:** When exposing data flows to the iOS UI, wrap Kotlin `StateFlow` structures inside an `SKIE` or multiplatform-compatible wrapper so Swift can consume them natively as async streams.
- **Build Validation:** Run `./gradlew :shared:linkReleaseFrameworkIosArm64` to verify that code builds correctly under the Kotlin/Native toolchain.
- **Information:** Always alert the user of anything they need to change in Xcode, assuming that XCode is not even set up if the user does not specify so previously.