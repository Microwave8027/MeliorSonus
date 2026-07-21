---
name: koin-best-practices
description: How to use koin for multiplatform
---
**Dependency Injection Specifications**
1. **App Module:** Specify modules in a app module as usual with classic kotlin DSL
   - Do not state components in the app module
   - Use factories for Use cases
   - Use singletons for repositories
   - State a function for using these app modules
2. **Main Activity** Use a globally stated initKoin function here
   - State context as usual
   - Pass platform-specific modules into the `initKoin` function to handle dependencies like the Android `Context` or database drivers.
