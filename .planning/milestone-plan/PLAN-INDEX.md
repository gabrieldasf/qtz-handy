# Plan Index: M1 Smart Correction History

## Phase Order

1. `M1-P1-correction-data-model`
2. `M1-P2-history-correction-ui`
3. `M1-P3-transcription-application`
4. `M1-P4-verification-polish`

## Lane Policy

Use three lanes per executable phase:

- `implementation`
- `verification`
- `integration-review`

If subagents are unavailable, run lanes sequentially and preserve separate evidence in phase artifacts.

## Planning Gaps

The canonical `$gsd-*` skills are not installed on this host. These artifacts are compatible fallback planning state. Before implementation, expand each phase into full `SPEC.md`, `CONTEXT.md`, and `PLAN.md`.

## Context7

Context7 returned an invalid API key. Implementation phases that touch React, Tauri, Specta, or plugin APIs must either restore Context7 or record official documentation fallback.
