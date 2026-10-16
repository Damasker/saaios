# ADR-846: recapture skip read_camp_owner_log

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `9643d0f3…` (ADR-845). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not tail the camp owner log.

## Нумерация

После ADR-845 следующий свободный номер — **846**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-844 skips `read_cp_state` on recapture through
`me_facts()`. Recapture still cloned `me_camp_fact_log` and
then called `apply_me_camp_log_probes`, so a missing camp
log still STREAMed the owner log. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices.
Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies held camp-log values and skips
   `apply_me_camp_log_probes`. The camp-log turn still
   STREAMs the owner log once. Do not tail it.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the camp-log ready arm has no
   `apply_me_camp_log_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_camp_log_probes` after cloning
`me_camp_fact_log` in `me_facts()`. Live `9643d0f3…` still
STREAMs the owner log on recapture when the held log is
missing.

## Ссылки

- ADR-844, ADR-845, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
