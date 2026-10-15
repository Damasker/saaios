# ADR-746: defer CP sysfs off the camp turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `95821052…` (ADR-745). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-745 следующий свободный номер — **746**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-745 fills wlan0 / hci0 after kernel / uptime. The camp
turn still read the modem CP state node together with the
`/proc` walk. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_camp_probes` does not call `read_cp_state`.
   `/proc` owner / endpoint fill without the CP node.
2. The next event-loop turn reads CP onto held facts and
   stores `me_cp_hold`. Locality JSON still waits so the
   owner-log stream sees ONLINE. A Wi-Fi connect drops
   `me_net_hold` only. Recapture skips CP when the token is
   held.
3. Host test: camp probes have no `read_cp_state`; CP turn
   stores the hold; idle blocks on `cp_pending`. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Call `read_cp_state` from `apply_me_camp_probes`. Live
`95821052…` still reads CP during camp.

## Ссылки

- ADR-708, ADR-744, ADR-745, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
