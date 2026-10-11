# ADR-481: VUI-09 — Система Android VM / notebook as Unavailable

## Статус

Принято, 2026-10-11. Handoff `surface-system` names Android VM and
notebook as RuntimeRows. No APP-COMPAT runtime is installed. Rows
are Status SettingRows «Недоступно · нет среды». Never Ready /
Готово. No probe. Battery gauge stays omitted (ADR-126).
Experimental. PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-480 следующий свободный номер — **481**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-126 omitted these rows so a painted green «Готово» could not
ship. ADR-470 reopened board chrome. The board now wants the
honest Unavailable reason, not a hidden future.

## Decision

1. **Host-lock** Система section «Среды» with two Status
   `SettingRow`s: «Android VM» and «Блокнот».
2. **Live.** Status only. No interned action. Copy is «Недоступно
   · нет среды» until a real runtime capability exists.
3. **Not Ready.** Do not paint Готово, green health, or a launch
   button. Do not probe `saaios-runtime` (ADR-030).
4. **Not this slice.** Do not change `me-public.sui`. Do not add a
   battery gauge. Do not convert leftover keyboard / gallery /
   lock formulas.

## Consequences

- Система names the missing runtimes without faking them.
  Rollback: drop «Среды». Still not Visual v1 sign-off.

## Verification

Host: both rows exist, Status, no dispatch, no Готово; timezone
intern stays `cycle_timezone`. Panther: do not flash; leave Сейчас.
