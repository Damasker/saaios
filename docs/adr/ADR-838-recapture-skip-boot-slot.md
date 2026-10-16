# ADR-838: recapture skip boot_slot

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `e2f45dce…` (ADR-837). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not re-add `boot_attempts`. Do
not skip cmdline when bootconfig has a slot.

## Нумерация

После ADR-837 следующий свободный номер — **838**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-836 skips wlan0/hci0 `exists()` on recapture through
`me_facts()`. Recapture still applied `me_boot_hold` and then
called `apply_me_boot_probes`, so empty/`неизвестно`
`boot_slot` still read `/proc/bootconfig`. dest-no-lock stays
off. Do not start the shell from COM13. Do not open modem
devices. Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies `me_boot_hold` and skips
   `apply_me_boot_probes`. The boot turn still reads
   `/proc/bootconfig` first, then cmdline only if the slot is
   missing.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Do not call `boot_attempts`.
3. Host test: the boot hold arm has no
   `apply_me_boot_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_boot_probes` after applying `me_boot_hold` in
`me_facts()`. Live `e2f45dce…` still reads `/proc/bootconfig`
on recapture when `boot_slot` is empty or `неизвестно`.

## Ссылки

- ADR-836, ADR-837, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
