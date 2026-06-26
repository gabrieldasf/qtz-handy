# PLAN: Verification And Polish

## Implementation Lane

- Fill missing i18n keys.
- Adjust layout polish after manual UAT.
- Update docs or planning notes if the final behavior differs from this milestone.

## Verification Lane

- Run formatting, lint, build, and targeted tests.
- Run manual UAT:
  - create correction from History;
  - create multiple rows with keyboard only;
  - verify future transcription/pipeline application using test or controlled sample.

## Integration-Review Lane

- Review changed files for unrelated edits.
- Confirm no generated artifacts are committed unless intentionally tracked.
- Confirm `.planning/` remains versioned and useful for resume.
