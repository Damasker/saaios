# ADR-684: small supervisor boot-log tail

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `bb350461…` (ADR-683). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-683 следующий свободный номер — **684**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-683 reads the camp owner log once. The live-probe turn
still pulled 256 KiB from `/run/modem-boot.log` to parse one
`supervise=` token. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not log the
boot file.

## Решение

1. `read_supervisor_token` takes an 8 KiB tail. The parser
   still walks lines from the end and accepts only the stock
   tokens (`hold`, `owner-gone`, `cp-left`, `attend`,
   `launch-once`).
2. Host test: `hold` from a padded fixture; empty text is
   none. Do not flash this ADR. dest-no-lock stays off.

## Откат

Restore the 256 KiB tail. Live `bb350461…` still reads that
window on every live-probe turn.

## Ссылки

- ADR-682, ADR-683, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
