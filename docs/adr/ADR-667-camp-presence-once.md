# ADR-667: one camp presence snapshot per Me facts

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `ed2bc0d1…` (ADR-666). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-666 следующий свободный номер — **667**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-666 settled Sistema after chrome + deferred facts. The
facts turn still walked `/proc` once per cellular field:
`camp_log_is_current()` calls `camp_owner_running()` (full
cmdline scan) and `read_cp_state()` about forty times, then
again for owner/action/open. First Sistema fill stayed
CPU-heavy for tens of seconds.

## Решение

1. `camp_log_current_from(owner, cp)` is the predicate.
   `camp_log_is_current` is gone — it walked `/proc` per
   field.
2. `me_facts` reads owner, CP, endpoint, and status-query
   once, then reuses them. Session suffix on the row uses
   those facts, not a new `/proc` walk.
3. Host test: current only when owner and `ONLINE`. Do not
   flash this ADR. dest-no-lock stays off. Do not log
   secrets.

## Откат

Revert the snapshot. Live `ed2bc0d1…` still fills facts with
one `/proc` walk per field.

## Ссылки

- ADR-666, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
