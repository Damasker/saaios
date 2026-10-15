# ADR-700: skip `/proc` cmdline unless `comm` is a camp binary

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `f9e2d372…` (ADR-699). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-699 следующий свободный номер — **700**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-694 stops the `/proc` walk after owner, `sit-sim-status`,
and `saai-modemd`. Until then every pid still opens
`cmdline`. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `proc_comm_is_camp_candidate` keeps `saai-modemd`,
   `sit-sim-status`, and `modem-rfs-camp*` (16-byte `comm`
   truncation). Other names skip `cmdline` and fd.
2. The live walk reads `comm` first. Classify still uses
   cmdline bytes, including `saai-modemd --ipc-fd` as owner.
3. Host test: camp names pass; `kworker` and `saai-shell` do
   not. Do not flash this ADR. dest-no-lock stays off.

## Откат

Drop the `comm` gate. Live `f9e2d372…` still reads every
`/proc/<pid>/cmdline` until the three processes are known.

## Ссылки

- ADR-694, ADR-699, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
