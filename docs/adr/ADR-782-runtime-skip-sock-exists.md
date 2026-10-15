# ADR-782: runtime skip unix sock exists

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `2602a2af…` (ADR-781). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls.

## Нумерация

После ADR-781 следующий свободный номер — **782**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-780 already skips `/tmp/saaios.sock` on aarch64. The
runtime turn still stated each candidate with `exists()`
before `connect`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type
PSK. One status JSON read stays.

## Решение

1. `read_runtime_live_facts` connects the unix candidate. It
   does not call `path.exists()`. `NotFound` tries the next
   candidate. Any other connect error returns empty facts.
2. TCP fallbacks stay after the unix loop. A Wi-Fi connect
   drops `me_net_hold` only. Recapture skips JSON when
   runtime is held.
3. Host test: the helper has no `path.exists()`. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Call `path.exists()` before `UnixStream::connect`. Live
`2602a2af…` still stats the sock on every runtime turn.

## Ссылки

- ADR-770, ADR-780, ADR-781, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
