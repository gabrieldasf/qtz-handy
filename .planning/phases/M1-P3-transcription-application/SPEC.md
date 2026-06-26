# SPEC: Transcription Application

## Objective

Make learned corrections affect future transcription output reliably.

## Requirements

- Apply deterministic rules after ASR text is produced and before paste/history final output.
- Feed desired correction terms into ASR vocabulary context where supported.
- Include correction context in LLM post-processing when post-processing is enabled.
- Preserve original transcript history fields or make source/final fields clear.
- Avoid over-correcting unrelated text.

## Acceptance Criteria

- Given rule `Live Zap -> Livess App`, future output containing `Live Zap` becomes `Livess App`.
- Phrase corrections work across punctuation boundaries where reasonable.
- Disabled rules are ignored.
- Whisper and non-Whisper paths both have a defined behavior.
