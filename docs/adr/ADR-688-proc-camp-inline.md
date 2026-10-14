# ADR-688: classify `/proc` camp presence inline

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `04308074…` (ADR-687). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-687 следующий свободный номер — **688**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-680 walks `/proc` once, then materializes every pid
cmdline into a `Vec` before classify. The live-probe turn
still allocates that list. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices.

## Решение

1. `proc_camp_presence_apply` updates one scan from one row.
   The live walk classifies each cmdline and drops it.
2. `proc_camp_presence_from_rows` stays for the host fixture.
   Endpoint still comes from fd symlink names only.
3. Host test: owner + `sit-sim-status` + endpoint without a
   collected list. Do not flash this ADR. dest-no-lock stays
   off.

## Откат

Restore the collected `Vec`. Live `04308074…` still allocates
every `/proc` cmdline on the live-probe turn.

## Ссылки

- ADR-680, ADR-681, ADR-687, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
