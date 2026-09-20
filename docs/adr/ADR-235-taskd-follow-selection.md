# ADR-235: `saai-taskd` follows entityd `SelectionChanged`

## Статус

Принято, 2026-09-20. ADR-233/234 start `saai-taskd` with `--space`
from `selection.json` at exec. The daemon then ignored
`SelectionChanged`, so Home/personal intents stalled while the
process stayed on Work. This slice retargets the watch. PIN stays
null. Do not tap Spaces rows. Not Visual v1 sign-off. WORK-02 stays
host-only.

## Нумерация

После ADR-234 следующий свободный номер — **235**. Не S33.

## Контекст

`saai-entityd` already emits `EntitydEvent::SelectionChanged` on
`select_space`. `saai-taskd::run` matched only `EntityChanged` and
`continue`d everything else. Boot `--space work` was correct for this
device's `selection.json`, but a later selection did not move the
daemon. Goal wave A2: watch the selected space, not only Work.

## Decision

1. **Follow.** On `SelectionChanged`, if `space_id` differs, replace
   the watch, reload that space's tasks, reconcile intents and
   confirmed tasks. Same-space events are a no-op.
2. **Still one space.** Do not subscribe to every space. In-flight
   work in the previous space stays there until selection returns.
3. **No shell tap.** Host tests use a fake entityd. Device proof may
   use the entityd socket `select_space` command, not a Spaces row.

## Consequences

- Intents in Home/personal are processed after that space is
  selected. Rollback: drop `follow_selected_space` and ignore
  `SelectionChanged` again. WORK-02 live dispatch stays A4. Still not
  Visual v1 sign-off.

## Verification

Host: `cargo test -p saai-taskd` 71 passed, including
`follow_selected_space_retargets_the_watch`. Panther: binary
`2d3224af…` pid 591; socket `select_space` (not a Spaces row) logged
`follow space work -> home` and `follow space home -> work`. Restored
`selection.json` to `work`. Marker on. Do not tap
Spaces/Разрешить/Сопряжь.
