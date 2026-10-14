# ADR-676: first Sistema list skips live probes

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `4e18d857…` (ADR-675). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-675 следующий свободный номер — **676**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-675 settled `S` after `surface=me`. The first real list
still blocked on one `me_facts()`: `wpa_cli` (no timeout),
`df /data`, three `/proc` walks, camp reads, and the runtime
status socket. dest-no-lock stays off. Do not start the shell
from COM13.

## Решение

1. First capture uses `me_facts_chrome`: settings, identity,
   and sysfs only. Storage stays «неизвестно». Wi-Fi stays
   «Нет статуса». Cellular fields stay empty. No `wpa_cli`,
   `df`, `/proc` owner walks, or runtime JSON.
2. The next event-loop turn sets `me_full_ready` and runs the
   live `me_facts()`. Observations still wait one more turn
   (ADR-672). Idle stays blocked while `me_full_pending`.
3. Host test: chrome facts omit «Наблюдения» and keep the
   existing empty copies. Do not flash this ADR.

## Откат

Revert `me_facts_chrome` / `me_full_pending`. Live
`4e18d857…` still waits on `wpa_cli`/`df` before the first
Sistema list.

## Ссылки

- ADR-672, ADR-674, ADR-675, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
