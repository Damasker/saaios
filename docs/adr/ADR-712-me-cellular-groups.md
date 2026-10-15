# ADR-712: group Sistema cellular facts under Связь

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `3be0c132…` (ADR-711). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-711 следующий свободный номер — **712**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

«Сотовая сеть» joined camp tokens into one status line.
Healthy summary and DNS / IPv4 / VoNR / Трасса sat in the
same string. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Решение

1. `Сотовая сеть` keeps CP, registration, radio, SIM, process,
   endpoint, supervisor, action, open, ifaces, and the session
   suffix.
2. Remaining camp tokens become readout rows under Связь.
   Labels come from `observation_row_label` (DNS, IPv4, IPv6,
   ENDC, SMSC, VoNR, Трасса, GPS, PS, SA). No new names.
   `cellular.screen` / `cellular.linkcrit` stay off chrome.
3. Host test: Связь still owns the grouped rows; the summary
   does not contain `dns `. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Join every camp token back into `Сотовая сеть`. Live
`3be0c132…` still dumps them on one row.

## Ссылки

- ADR-531, ADR-711, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-06 / VUI-09
