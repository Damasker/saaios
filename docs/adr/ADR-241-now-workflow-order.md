# ADR-241: Сейчас leads with attention, current work, next

## Статус

Принято, 2026-09-20. NOW body order is chrome paint; panther
flash followed host-green. PIN stays null. Not Visual v1 sign-off.

## Нумерация

После ADR-240 следующий свободный номер — **241**. Не S33.

## Контекст

VUI-03 painted `Сегодня` first, then `Продолжается`, then
`Требует внимания`, then `Далее`. The boards' Сейчас job is
attention, current work, next action, composer. A calendar-first
home is a different product. Goal wave B, one screen. Do not
invent weather, voice, or graphs. Empty sections stay omitted.

## Decision

1. **Order.** `now_workflow_sections` emits `Требует внимания`,
   `Продолжается`, `Далее`, then enabled `saaios.schedule` as
   `Сегодня` when that source exists.
2. **Composer.** Orb + intent footer stay the composer. This
   slice does not enlarge Orb or remove the apps footer.
3. **Sources.** Rows still come from the attention projection,
   running tasks, pending Action / derived-ready Task, and
   enabled schedules. No new entity types.

## Consequences

Object View honesty, Space detail, and Orb workflow states stay
later B slices. Rollback: restore Сегодня-first. Visual v1 still
unsigned.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+267 passed, including
`now_sections_lead_with_attention_then_work_then_next` and
`now_sections_omit_empty_workflow_blocks`.
Panther: shell `66db9031…` pid 1401. Leave Сейчас; do not tap
Поиск, Inbox, Spaces, Разрешить. Marker on.
