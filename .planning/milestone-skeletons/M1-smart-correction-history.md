# M1 Smart Correction History

## Outcome

Users can correct transcription mistakes directly from History and teach Handy reusable local correction rules.

## User Workflow

The core interaction is optimized for repeated keyboard entry:

```text
Add correction
wrong text -> Tab/Enter/ArrowRight -> correct text -> Tab/Enter/ArrowRight -> new row
```

Example:

```text
Live Zap -> Livess App
Quartzo show runner -> Quartzo Showrunner
```

## Product Requirements

- Add correction UI inside History, close to each transcript.
- Allow phrase-level corrections, not only single words.
- Preserve speed for repeated row entry.
- Persist corrections locally.
- Apply corrections automatically to future transcriptions.
- Make the correction source auditable from History and settings.
- Reuse the existing Handy settings UI design system.

## Technical Requirements

- Add a typed correction model.
- Add persistence, likely SQLite for correction rules or app settings for a smaller first pass.
- Add Tauri commands and Specta bindings.
- Add frontend state and keyboard handling.
- Integrate with transcription output before paste and history save.
- Keep existing `custom_words` behavior compatible.
- Add tests for correction matching and persistence.

## Risks

- Prompt-only correction is non-deterministic.
- Over-aggressive fuzzy matching can corrupt valid text.
- Keyboard handling can fight browser defaults if implemented too broadly.
- Existing `custom_words` UI currently blocks spaces.
- History retention can delete audio/transcript context; correction rules must not depend on retained audio.

## Exit Criteria

- User can add multiple correction rows from History using only keyboard after the first click.
- Future transcriptions apply stored rules.
- Build, lint, and targeted backend tests pass.
- Manual UAT proves the row flow.
