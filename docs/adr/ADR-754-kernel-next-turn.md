# ADR-754: defer kernel release off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `79e1c0fb…` (ADR-753). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-753 следующий свободный номер — **754**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-753 fills device-tree model after kernel. The chrome turn
still read `/proc/sys/kernel/osrelease`. dest-no-lock stays
off. Do not start the shell from COM13. Do not open modem
devices. Do not type PSK. Section «Связь» stays. Device
«Экран» stays. Node identity line `panther · aarch64` stays.

## Решение

1. `apply_me_chrome_probes` does not call `kernel_release`.
   Chrome has no remaining chrome sysfs.
2. The next event-loop turn reads the kernel onto held facts
   and stores `me_kernel_hold`. Model still waits. A Wi-Fi
   connect drops `me_net_hold` only. Recapture skips kernel
   when the string is held.
3. Host test: chrome probes have no `kernel_release()`; kernel
   turn stores the hold; idle blocks on `kernel_pending`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `kernel_release` from `apply_me_chrome_probes`. Live
`79e1c0fb…` still reads osrelease during chrome.

## Ссылки

- ADR-726, ADR-752, ADR-753, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
