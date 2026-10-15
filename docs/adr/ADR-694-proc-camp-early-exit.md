# ADR-694: stop the `/proc` camp walk once owner, status, and modemd are known

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `f25649f7…` (ADR-693). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-693 следующий свободный номер — **694**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-688 classifies each `/proc` cmdline inline. The live walk
still reads every pid even after owner, `sit-sim-status`, and
`saai-modemd` are already known. Stopping at owner plus
endpoint would report `owner` instead of `shared`. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK.

## Решение

1. `ProcCampScan` records that `saai-modemd` exists, separate
   from whether it holds an endpoint. `proc_camp_scan_complete`
   is owner and status-query and modemd.
2. The live walk and the row fixture break at that point. A
   later modemd fd does not turn `owner` into `shared`.
3. Host test: complete after the three processes; leftover
   shared fd stays `owner`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Drop `proc_camp_scan_complete`. Live `f25649f7…` still walks
every `/proc` pid on the camp turn.

## Ссылки

- ADR-688, ADR-693, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
