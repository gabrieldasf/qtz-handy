# CONTEXT: Transcription Application

## Relevant Files

- `src-tauri/src/managers/transcription.rs`
- `src-tauri/src/actions.rs`
- `src-tauri/src/audio_toolkit/text.rs`
- `src-tauri/src/llm_client.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/tray.rs`

## Current Behavior

The transcription pipeline currently:

1. transcribes audio with the selected engine;
2. applies `custom_words` correction for non-Whisper engines;
3. filters filler/hallucination text;
4. optionally runs LLM post-processing;
5. saves to history and pastes output.

## Design Direction

Correction rules should run as deterministic text transforms before paste. LLM post-processing may improve text quality, but cannot be the source of truth for correction reliability.
