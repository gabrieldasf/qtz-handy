# CONTEXT: Verification And Polish

## Relevant Commands

- `bun run format:check`
- `bun run lint`
- `bun run build`
- targeted `cargo test` under `src-tauri`

## Known Environment Notes

On this Windows machine, successful Tauri build used:

- portable CMake;
- portable LLVM/libclang;
- portable Vulkan SDK;
- short `CARGO_TARGET_DIR=C:\t\qh` to avoid MSBuild path length issues.

These are local environment details, not app requirements.
