# ADR-190: VUI-09 — increased text on panther without a Me tap

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`. `text_scale_pct`
went 100 → 150 → 100 through `shell-settings.json` and an unlocked
shell restart. No Me tap, so no 7-tap, no lock-timeout row, no PIN,
no Orb. `compile()` stays `sui 1`. No new daemon. Leave Сейчас.

## Нумерация

После ADR-189 следующий свободный номер — **190**. Не S33.

## Контекст

The ledger left «increased text on panther» open: host `set_text_scale`
existed, ADR-110 had an older 150% shot, HEAD did not. Tapping
«Размер текста» on Система sits under the 7-tap build row and the
lock-timeout cycles. Reloading the setting from JSON is the same
path `load_settings` already uses at start.

## Decision

1. **Do not tap Система.** Write `"text_scale_pct": 150`, kill the
   unlocked shell (marker on), shot Сейчас, restore `100`, kill again.
2. **Do not reflash.** Binary stays ADR-188.
3. **Do not lock, reboot, or kill displayd.**

## Consequences

- 150% is proven on HEAD chrome. Rollback: keep `text_scale_pct` at
  100. Lock cycle, cold boot, and radio-off stay open.

## Verification

Host: scale cycle includes 150; ledger cites ADR-190. Panther: Сейчас
at 150 then restored 100; leave Сейчас.
