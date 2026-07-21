# Rule: KMP UI Guidelines
activation: glob
files: **/*

- Always use Compose Multiplatform components over platform-native views.
- Keep UI components stateless; hoist state up to ViewModel classes.
- Follow standard practices
- Always use stored sizes and colors to increase modularity(not 20.dp/sp or 0xfffff)
- Always use a global material theme

# Rule: Structuring
activation: glob
files: commonMain

- Add folders for every different part of the app respectively within the ui, domain, and data directories
- Add a core folder for network requests and DI, databases or datastores, etc. Anything that doesn't fall in the ui, domain or data layers.
- Use constructor injection for all dependencies to facilitate testing and modularity.
- Ensure all business logic resides in the domain layer using UseCases or Interactors.

# Rule: Execution Permission
activation: glob
files: **/*

 - Never write new app logic before confirming with the user.
 - You are allowed to modify dependencies within the build configuration files.

# Rule: IOS code generation
activation: glob
files: iosApp

- Minimize logic in the Swift layer; delegate all business logic to the shared KMP module.
- Use SwiftUI for the UI layer while observing shared ViewModels or StateFlows.
- Ensure proper mapping between Kotlin and Swift types, especially for Enums and Sealed Classes.
- Use direct integration with a plugin to handle Sealed Classes and Coroutines (Flows) seamlessly in Swift.
- Handle platform-specific resources (strings, images) using the shared MR (Multiplatform Resources) accessors.