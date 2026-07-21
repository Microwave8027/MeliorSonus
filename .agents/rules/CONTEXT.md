# Project Overview: Music Education Trainer App

## 1. App Summary & Core Intent
This is a Kotlin Multiplatform (KMP) mobile application designed to provide real-time audio analysis and vocal/performance feedback. It captures microphone input, processes the audio signal, and displays live visual feedback to the user. It also uses a display from a library known as OSMD(Open Sheet Music Display) which uses javascript to render a display. Once the user gets passed the authentication screen(don't implement this), it will see a scaffold style ui with a bottom bar that shows the home and profile tabs. There will also be a lazyColumn displaying all the user's sheet music as well as a button in the bottom right as a plus button that when pressed, allows the user to search up music upon pressing enter which will fetch from my custom api endpoint. Afterward, the user will see a pdf preview. After the user selects a piece they like, they will be taken to a osmd rendered music displayer from musicXML. Specifics on how information will be passed from and to the backend and features will be specified by through prompting later on. There will also be an audio engine that actively listens to the user and maps the sound into a ring buffer. Further logic will be applied later.
## 2. Technical Architecture & Tech Stack
- **Language:** Kotlin (Kotlin Multiplatform)
- **UI Framework:** Compose Multiplatform
- **Audio Processing:** Custom `AudioProcessor` implementation (platform-specific for iOS/Android)
- **Dependency Injection:** Koin with classic kotlin DSL
- **Testing:** Kotlin Test, Compose Test Lab
- **Navigation:** Decompose
- **Datastore:** Proto Datastore
- **Database:** SQLDelight
- **Networking:** Ktor
- **Concurrency:** Kotlin Coroutines & Flow
- **Serialization:** Kotlinx Serialization
- **Sheet Music Display:** Webview with OSMD
- **Integration Method:** Direct integration with swift IOS

## 3. Development Rules
- Never write new app features before confirming with the user.
- Maintain at least 80% code coverage for new features
- You are allowed to modify dependencies within the build configuration files.
- Implement a little bit of the app at a time according to user request rather all at once.
- Always generate interfaces for every class created for further modularity. Call domain and database layers "(...)Impl" for the actual implementations and call decompose components "Default(...)"
- Create private vals for re-usability on certain variables/functions.

## 4. Critical Folder Layout & Directory Rules
- **Shared Source Logic:** All shared code lives inside the `/shared` module.
- **UI Components:** Shared Compose UI views must go inside `shared/src/commonMain/kotlin/`.
- **Database Schemas:** All SQLDelight `.sq` query files must strictly go in `shared/src/commonMain/sqldelight/com/yourdomain/app/database/`.
- **DataStore Schemas:** All Protocol Buffer `.proto` files must strictly go in `shared/src/commonMain/proto/com/yourdomain/app/`.

## 5. Engineering Standards & Constraints
- **State Management:** Always use UI State production models paired with clean MVI or MVVM architecture patterns.
- **Thread Safety:** Real-time audio processing callbacks must remain entirely isolated on background dispatcher threads; never block the main UI thread.
- **Dependencies:** Prefer official multiplatform-ready libraries over platform-specific implementations unless native hardware frameworks are absolutely necessary.
