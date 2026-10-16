# ADR-828: recapture skip kernel_release

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `58533602…` (ADR-827). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`.

## Нумерация

После ADR-827 следующий свободный номер — **828**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-826 skips `df` on recapture later probes. Recapture still
builds facts through `me_facts()`, which applied
`me_kernel_hold` and then called `apply_me_kernel_probes`, so
an empty kernel string still re-read
`/proc/sys/kernel/osrelease`. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do not
type PSK. Leave Сейчас. Do not tap Orb. Do not 7-tap.

## Решение

1. `me_facts()` applies `me_kernel_hold` and skips
   `apply_me_kernel_probes`. The kernel turn still reads
   osrelease.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the kernel hold arm has no
   `apply_me_kernel_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_kernel_probes` after applying `me_kernel_hold`
in `me_facts()`. Live `58533602…` still reads osrelease on
recapture when the kernel string is empty.

## Ссылки

- ADR-826, ADR-827, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
