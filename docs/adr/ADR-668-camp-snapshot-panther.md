# ADR-668: camp snapshot on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`1bb214c6…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 8960. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=shm` seq=6. Process settled `S`. Unlock stays the
proven path (ADR-564). Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-667 следующий свободный номер — **668**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-667 snapshots camp presence once per `me_facts`. Live
panther still had `ed2bc0d1…` (ADR-666). dest-no-lock stays
off. Do not start the shell from COM13. Do not mix modem WIP.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr666` (`ed2bc0d1…`).
3. Kill 6662 only. displayd 2694 respawned 8960.
4. Unlock then Sistema. Chrome committed, facts filled,
   process went `S`. dest-no-lock stayed absent. First fill
   still takes tens of seconds; that is paint, not `/proc`.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr666 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-666, ADR-667, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
