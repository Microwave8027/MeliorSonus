--
name: sqldelight-compiler
description: Compiles and validates SQLDelight schemas across KMP modules.
---
# Verification Protocol
When asked to create or update database tables:
1. Write the new `.sq` file to `shared/src/commonMain/sqldelight/com/yourdomain/app/database/`.
2. Immediately execute the local Gradle task to verify the compiler schema generation:
   `./gradlew :shared:generateSqlDelightInterface`
3. If the build logs throw any namespace or query validation errors, immediately catch the stack trace, fix the raw SQL file, and re-run.
4. Implement the `expect`/`actual` `DatabaseDriverFactory` in other mains