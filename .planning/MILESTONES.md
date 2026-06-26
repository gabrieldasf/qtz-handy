# Milestones

## Active Milestone: M1 Smart Correction History

**ID:** `M1-smart-correction-history`

**Goal:** Let users turn mistakes found in History into reusable correction rules, with a fast keyboard-first row editor:

1. User opens a transcript in History.
2. User clicks `Add correction`.
3. User types the misheard text.
4. `Tab`, `Enter`, or `ArrowRight` moves focus to the correction field.
5. User types the desired text.
6. `Tab`, `Enter`, or `ArrowRight` saves the row and opens a new empty row below.
7. The correction is reused in future transcriptions.

**Primary example:** `Live Zap -> Livess App`

**Truth-teller decision:** This is a correction rule, not just a custom word. The durable behavior should combine deterministic replacements with ASR vocabulary hints and optional LLM post-processing. Prompt-only correction is useful context, but it is not reliable enough as the only mechanism.

## Acceptance Criteria

- History entries expose an easy `Add correction` affordance without disrupting copy/save/retry/delete.
- Correction entry supports keyboard-only flow:
  - wrong field to correct field via `Tab`, `Enter`, or `ArrowRight`;
  - correct field commits and opens the next row via `Tab`, `Enter`, or `ArrowRight`;
  - empty rows do not create rules;
  - duplicate rules are handled clearly.
- Corrections support phrases with spaces.
- Corrections persist locally.
- Future transcriptions apply rules after ASR output and before paste.
- Whisper receives desired vocabulary as context where safe.
- Existing custom words keep working or are migrated cleanly.
- UI follows Handy design system and i18n rules.
- Validation includes frontend build/lint, backend tests for correction application, and manual UAT for the keyboard flow.

## Phases

1. `M1-P1-correction-data-model`
   - Add persistent correction rules and migrate/bridge existing `custom_words`.
2. `M1-P2-history-correction-ui`
   - Add History UI for keyboard-first correction rows.
3. `M1-P3-transcription-application`
   - Apply deterministic replacements, vocabulary hints, and optional LLM prompt context.
4. `M1-P4-verification-polish`
   - Add coverage, i18n, regression checks, and product polish.

## Out Of Scope

- Cloud sync.
- Teams/shared dictionary.
- Mobile apps.
- Full Wispr Flow command mode.
- App-aware styles.
- Upstream PR preparation.
- Deploy, signing, packaging, or release automation.
