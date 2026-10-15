# ADR-714: reuse Sistema runtime JSON; skip recapture read

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `daef927d…` (ADR-713). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-713 следующий свободный номер — **714**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-713 grouped Связь on panther. After the staircase dropped
the hold, recapture still read runtime `status` JSON for
identity, «Наблюдения», and memory. dest-no-lock stays off.
Do not start the shell from COM13. Do not open modem devices.
Do not type PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_runtime_probes` returns when observations or
   memory records are already held. It does not call
   `read_runtime_live_facts`.
2. The runtime turn stores those fields on `me_runtime_hold`.
   Recapture seeds facts from that cache before apply.
3. Host test: held CPU observation / model survive apply.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Always call `read_runtime_live_facts` from runtime probes.
Live `daef927d…` still reads JSON on recapture.

## Ссылки

- ADR-690, ADR-710, ADR-713, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
