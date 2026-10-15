# ADR-756: drop the empty chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `03012fd5…` (ADR-755). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-755 следующий свободный номер — **756**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-755 left chrome with no remaining sysfs. The extra event-loop
turn still marked chrome ready, blocked idle, and redrew
settings. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Связь» stays. Device «Экран» stays. `me_chrome_hold` still
owns «Местность».

## Решение

1. First Sistema flatten stays settings. The next loop is
   kernel (`kernel_release`), not an empty chrome turn.
2. `me_chrome_ready` flips on capture without I/O.
   `me_chrome_hold` still stores locality. A Wi-Fi connect
   drops `me_net_hold` only.
3. Host test: no `else if shell.me_chrome_pending`; idle has
   no `chrome_pending`; kernel turn still stores the hold. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Restore `me_chrome_pending` between facts and kernel. Live
`03012fd5…` still spends one loop on empty chrome.

## Ссылки

- ADR-726, ADR-754, ADR-755, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
