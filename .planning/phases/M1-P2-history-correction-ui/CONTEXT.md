# CONTEXT: History Correction UI

## Relevant Files

- `src/components/settings/history/HistorySettings.tsx`
- `src/components/settings/CustomWords.tsx`
- `src/components/ui/Button.tsx`
- `src/components/ui/Input.tsx`
- `src/components/ui/AudioPlayer.tsx`
- `src/i18n/locales/en/translation.json`
- other locale files under `src/i18n/locales/`

## Current Behavior

History entries render transcript text, audio player, and icon buttons for copy, save, retry, and delete.

The `CustomWords` UI currently allows only one non-space word up to 50 chars. This does not satisfy phrase-level corrections like `Live Zap -> Livess App`.

## Design Direction

Keep the UI compact inside each history entry. Use existing icon button style and current border/divider rhythm. Avoid a new visual system.
