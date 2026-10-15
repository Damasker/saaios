# ADR-698: apply camp probes onto held chrome Sistema facts

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `64c3fc50…` (ADR-697). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-697 следующий свободный номер — **698**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-676 fills chrome first, then the next turn should run camp
`/proc` and the owner log. Held chrome facts made
`capture_me_rows` skip `me_facts()`, so cellular rows stayed
empty on first open. dest-no-lock stays off. Do not start the
shell from COM13. Do not type PSK.

## Решение

1. `apply_me_camp_probes` writes camp fields onto existing
   facts. Kernel, uptime, boot, playback, and adapter sysfs
   stay from chrome.
2. The full-facts turn applies that onto `me_facts_held`
   before flatten, same shape as the Wi-Fi turn. Supervisor
   stays on the net turn.
3. Host test: chrome kernel/uptime survive; `ONLINE` / owner
   / `registration_raw` fill. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Drop `apply_me_camp_probes` from the full-facts turn. Live
`64c3fc50…` still skips camp when chrome facts are held.

## Ссылки

- ADR-676, ADR-697, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
