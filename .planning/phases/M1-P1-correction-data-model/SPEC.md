# SPEC: Correction Data Model

## Objective

Create the durable local model for learned correction rules.

## Requirements

- Represent a correction as `heard_text -> correct_text`.
- Support multi-word phrases.
- Store enabled/disabled state.
- Track timestamps for creation and updates.
- Avoid duplicate active rules.
- Preserve compatibility with existing `custom_words`.
- Expose typed Tauri commands and generated frontend bindings.

## Acceptance Criteria

- Backend can create, list, update, delete, and apply correction rules.
- Existing `custom_words` continues to work.
- Tests cover exact replacement, phrase replacement, duplicate handling, disabled rules, and case/punctuation behavior.

## Open Decisions

- Use SQLite table versus settings store.
- Whether `custom_words` remains separate from correction rules or becomes a derived vocabulary list.
