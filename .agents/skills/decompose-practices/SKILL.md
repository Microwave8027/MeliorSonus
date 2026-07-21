---
name: decompose-practices
description: switch from thinking in standard navigation practices and viewmodels to decompose practices
---
**Standard Decompose Practices**
1. **Viewmodel to Component shift:** Use decompose components instead of viewmodels. Pass in usecases and repositories as needed, as well as passing in component context each time and inheriting componentContext by ComponentContext.
2. **Main Activity:** Put the root content composable here and pass in the context with respective platform libraries.
3. **Component tree:** Generate a component for every ui screen, just as you would with viewmodels. Pass these components directly into each composable.
4. **Dependency Injection:** Pass koin component into each component and use inject() to inject usecases/repositories.
5. **Root Component:** Use a private sealed class configs for each component and create a private fun createChild that creates a child with the default component of the config depending on what the config is exactly. Generate a childStack as well as a navigation variable as needed here.