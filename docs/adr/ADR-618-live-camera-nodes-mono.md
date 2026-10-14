# ADR-618: live Sistema camera nodes paint MonoBody

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Not VUI-05 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-617 следующий свободный номер — **618**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-617 painted DevSurface build/kernel as MonoBody. Sistema
«Камера» still painted live `video*` nodes through
`SettingRow::readout` (Body). Those node names stay `video*`.
Empty «Нет узла захвата» stays Body. Copy stays off. Do not
invent preview. Leave Сейчас. Font compare stays a device
task. ADR-213 thin tuning is not this slice.

## Решение

1. `camera_setting_row` uses `with_technical_value` when nodes
   exist.
2. Missing capture stays the existing Body readout.

## Откат

Restore `SettingRow::readout` for both arms.

## Ссылки

- ADR-256, ADR-617
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
