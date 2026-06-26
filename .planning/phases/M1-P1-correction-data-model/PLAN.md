# PLAN: Correction Data Model

## Implementation Lane

- Add `CorrectionRule` type.
- Add persistence and migration.
- Add commands for CRUD operations.
- Add rule application helper.
- Generate/update TypeScript bindings.

## Verification Lane

- Run targeted Rust tests for correction logic.
- Run `bun run build` after bindings update.

## Integration-Review Lane

- Confirm no existing custom-word behavior regresses.
- Confirm storage choice is local-only and does not affect secrets/provider state.
- Confirm migrations are non-destructive.
