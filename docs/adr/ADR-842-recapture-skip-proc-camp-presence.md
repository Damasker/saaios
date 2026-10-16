# ADR-842: recapture skip proc_camp_presence

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `b739e416…` (ADR-841). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not tail the camp owner log.
Section «Связь» stays.

## Нумерация

После ADR-841 следующий свободный номер — **842**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-840 skips locality JSON on recapture through `me_facts()`.
Recapture still applied `me_camp_presence` and then called
`apply_me_camp_probes`, so a missing camp owner still scanned
`/proc`. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Leave
Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies `me_camp_presence` and skips
   `apply_me_camp_probes`. The camp turn still reads `/proc`
   with the comm filter and early-exit.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Do not tail the camp owner log.
3. Host test: the camp presence hold arm has no
   `apply_me_camp_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_camp_probes` after applying
`me_camp_presence` in `me_facts()`. Live `b739e416…` still
scans `/proc` on recapture when the camp owner is missing.

## Ссылки

- ADR-840, ADR-841, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
