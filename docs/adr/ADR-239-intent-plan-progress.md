# ADR-239: Intent Object View plan → progress → result

## Статус

Принято, 2026-09-20. Intent Object View painted one Task path. After
ADR-238 a Plan is several Tasks. This slice shows those Tasks as
plan and progress. PIN stays null. Not Visual v1 sign-off.

## Нумерация

После ADR-238 следующий свободный номер — **239**. Не S33.

## Контекст

The boards' Intent surface is Goal → plan → progress → result.
Object View already has related / activity / observation slots.
`primary_related_task` still drives status and confirmation. A
single Task stays the linear path. Two or more related Tasks are a
plan. Goal wave B, one screen. Do not invent workers, voice, or a
second task list.

## Decision

1. **Plan.** Two or more Tasks that realize the Intent set related
   to `План: A → B`, parents before children via
   `depends_on_task_ids` then `created_at`.
2. **Progress.** The same Tasks fill activity as
   `title — status` joined by ` · `. Status text is the existing
   `task_status_text`.
3. **Result.** Observation stays the existing Result summary from
   the primary lineage. No invented graph.
4. **One Task.** related / activity stay the linear
   Intent→Task→Action→Result caption. «Нет задачи» is unchanged.

## Consequences

Nav `Поиск` and Inbox demotion stay later B slices. Rollback: drop
the plan caption and keep the primary path. Visual v1 still unsigned.

## Verification

Host: `cargo test -p saai-shell --offline -- --test-threads=1` 263
passed, including
`object_view_content_for_an_intent_shows_plan_progress_from_related_tasks`.
Panther: shell `826adfec…` pid 1098. Leave Сейчас; do not type an
Intent; do not tap Разрешить / Inbox / Spaces rows. Marker on.
