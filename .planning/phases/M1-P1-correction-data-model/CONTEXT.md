# CONTEXT: Correction Data Model

## Relevant Files

- `src-tauri/src/settings.rs`
- `src-tauri/src/managers/history.rs`
- `src-tauri/src/managers/transcription.rs`
- `src-tauri/src/audio_toolkit/text.rs`
- `src-tauri/src/lib.rs`
- `src/bindings.ts`

## Current Behavior

- `custom_words` is a `Vec<String>` in settings.
- Whisper receives `custom_words.join(", ")` as `initial_prompt`.
- Non-Whisper engines run `apply_custom_words` after transcription.
- History is stored in SQLite at app data `history.db`.

## Design Direction

Use deterministic correction rules for `heard_text -> correct_text`. Continue to send desired terms as ASR vocabulary context, but do not rely on prompts as the only correction mechanism.
