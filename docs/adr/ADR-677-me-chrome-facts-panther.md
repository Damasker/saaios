# ADR-677: chrome Sistema list on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`36a07194…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 14277. Unlock then
Sistema: `switched to Me`, chrome list frame `62e27a1d…`,
FramePace `surface=me` `backend=dmabuf` seq=9. Live probes
ran after that list. Process later settled `S`. Unlock stays
the proven path (ADR-564). Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance.
Wrap Acceptance stays unchecked.

## Нумерация

После ADR-676 следующий свободный номер — **677**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-675 still waited on `me_facts()` before the first
settings list. ADR-676 skips `wpa_cli`/`df`/`/proc` on that
first capture. dest-no-lock stays off. Do not start the shell
from COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr675` (`4e18d857…`).
3. Kill 12356 only. displayd 2694 respawned 14277.
4. Unlock (540,1200) then Sistema (945,2250). Chrome list
   committed before live probes. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr675 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-675, ADR-676, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
