# SPEC: History Correction UI

## Objective

Add a fast, keyboard-first `Add correction` workflow to History.

## Requirements

- Reuse existing Handy UI components and settings layout.
- Add an affordance on each history entry.
- Show correction rows with two fields: heard text and correct text.
- Keyboard flow:
  - in heard field, `Tab`, `Enter`, or `ArrowRight` focuses correct field;
  - in correct field, `Tab`, `Enter`, or `ArrowRight` saves the row and creates a new empty row below;
  - new row starts focused in heard field;
  - empty or partial rows do not save;
  - `Escape` cancels the active row.
- Support repeated entry without mouse interaction after first click.
- Use i18next for every user-facing label, tooltip, placeholder, and toast.

## Acceptance Criteria

- Manual UAT can enter at least three corrections in sequence using only keyboard.
- Duplicate and invalid rows produce clear feedback.
- Existing history actions still work.
