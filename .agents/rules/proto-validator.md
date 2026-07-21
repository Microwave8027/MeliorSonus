---
name: wire-proto-validator
description: Standard workflow for updating Protobuf schemas using the Wire plugin.
---
# Schema Update Protocol
1. Modify or add `.proto` files strictly within `shared/src/commonMain/proto/com/yourdomain/app/`.
2. Ensure package names in the `.proto` file match the directory structure.
3. Trigger class generation by running: `./gradlew generateMainWireKotlin`
4. Verify that the generated data classes are properly exposed and retrievable inside your shared KMP target architecture.
5. Implement the `expect`/`actual` `DatabaseDriverFactory` in other mains.
6. Run `./gradlew build` to ensure cross-platform compatibility and that all generated code is correctly linked.