# GSD Project: QTZ Handy

## Repo

- Target repo: `C:\Apps\QTZ-Apps\qtz-handy`
- Remote: `gabrieldasf/qtz-handy`
- Upstream: `cjpais/Handy`
- Branch: `main`

## Product Context

Handy is a local-first desktop speech-to-text app built with Tauri, Rust, React, TypeScript, Tailwind, and i18next.

The current app already has:

- transcription history with audio playback, copy, save, delete, and retry;
- custom words persisted in settings;
- Whisper `initial_prompt` support from custom words;
- fuzzy custom-word correction for non-Whisper engines;
- optional LLM post-processing with configurable providers and prompts.

## GSD Scope

This planning state covers QTZ-specific product work on the fork. It does not change upstream policy, release packaging, deployments, secrets, billing, or production provider state.

## Design System Constraints

Frontend work must reuse the existing Handy UI system:

- React functional components and hooks;
- Tailwind classes consistent with current settings screens;
- existing shared components such as `Button`, `Input`, `SettingContainer`, `AudioPlayer`, and settings group layout;
- Lucide icons for actions;
- i18next keys for all user-facing text;
- no hardcoded JSX strings.

## Validation Baseline

Use repo commands:

- `bun run build`
- `bun run lint`
- `bun run format:check`
- targeted Rust tests when backend logic changes
- manual UAT for keyboard flow in the history correction UI
