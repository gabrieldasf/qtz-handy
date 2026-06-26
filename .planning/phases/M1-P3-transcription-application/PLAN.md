# PLAN: Transcription Application

## Implementation Lane

- Load correction rules during transcription output processing.
- Apply rules in a deterministic helper.
- Add vocabulary hint generation from correction `correct_text` values.
- Extend post-processing prompt context only when post-processing is enabled.
- Ensure history records enough evidence to understand original versus corrected output.

## Verification Lane

- Add Rust tests for rule application.
- Run targeted cargo tests.
- Run `bun run build`.

## Integration-Review Lane

- Review order of operations against custom words and LLM post-processing.
- Check privacy posture stays local except when user already enabled LLM post-processing.
- Check no provider secrets are logged.
