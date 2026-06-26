# PLAN: History Correction UI

## Implementation Lane

- Add a small `Add correction` action to `HistoryEntryComponent`.
- Add a correction row editor component using existing `Input` and `Button`.
- Implement keyboard transition and row commit behavior.
- Wire to correction rule commands.
- Add i18n keys.

## Verification Lane

- Run `bun run lint`.
- Run `bun run build`.
- Manual UAT:
  - click `Add correction`;
  - type `Live Zap`;
  - press `Tab`;
  - type `Livess App`;
  - press `Enter`;
  - verify a new row appears focused for the next wrong phrase.

## Integration-Review Lane

- Check focus management does not break copy/save/retry/delete.
- Check text does not overflow on narrow settings window.
- Check all user-facing text uses i18n.
