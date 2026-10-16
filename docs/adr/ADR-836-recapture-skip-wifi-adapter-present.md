# ADR-836: recapture skip wifi_adapter_present

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `166089ca…` (ADR-835). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not invent an adapters split
turn.

## Нумерация

После ADR-835 следующий свободный номер — **836**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-834 skips tinyplay `exists()` on recapture through
`me_facts()`. Recapture still applied `me_adapters_hold` and
then called `apply_me_adapters_probes`, so absent wlan0/hci0
still `exists()` both adapters. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices.
Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies `me_adapters_hold` and skips
   `apply_me_adapters_probes`. The adapters turn still
   `exists()` wlan0 and hci0 together.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Net still reads
   `facts.wifi_present`, not `exists()` wlan0.
3. Host test: the adapters hold arm has no
   `apply_me_adapters_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_adapters_probes` after applying
`me_adapters_hold` in `me_facts()`. Live `166089ca…` still
`exists()` wlan0/hci0 on recapture when both adapters are
absent.

## Ссылки

- ADR-834, ADR-835, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
