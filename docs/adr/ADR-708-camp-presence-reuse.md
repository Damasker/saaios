# ADR-708: reuse camp `/proc` presence; skip recapture walk

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `a74c3dc4…` (ADR-707). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-707 следующий свободный номер — **708**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-707 reuses streamed owner-log lines. Recapture still
walked `/proc` and CP sysfs after the hold dropped.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK.

## Решение

1. `apply_me_camp_probes` returns when `cellular_owner` is
   already held. It does not walk `/proc` or read CP.
2. The camp turn stores presence on `me_camp_presence`.
   Recapture seeds facts from that cache before apply.
3. Host test: held `ONLINE` / owner survive apply. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Always call `proc_camp_presence` from camp probes. Live
`a74c3dc4…` still walks `/proc` on recapture.

## Ссылки

- ADR-700, ADR-707, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
