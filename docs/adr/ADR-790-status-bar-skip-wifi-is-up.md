# ADR-790: status bar skip wifi_is_up

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `622864fd…` (ADR-789). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb. Do not un-group Связь.

## Нумерация

После ADR-789 следующий свободный номер — **790**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-758 already skips `wlan0` exists on the net turn. The
status bar still read `/sys/class/net/wlan0/operstate` on
every refresh during first-open Sistema. dest-no-lock stays
off. Do not start the shell from COM13. Do not open modem
devices. Do not type PSK.

## Решение

1. `present_status_bar` uses held `wifi_status`. It does not
   call `wifi_is_up`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the status painter has no `wifi_is_up()`. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Call `wifi_is_up` from `present_status_bar`. Live `622864fd…`
still stats `wlan0` operstate on every status refresh.

## Ссылки

- ADR-758, ADR-789, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
