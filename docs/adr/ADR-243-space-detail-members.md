# ADR-243: Space detail lists SOM members, not invented people

## Статус

Принято, 2026-09-20. Spaces chrome/hit-test change; panther flash
followed host-green. PIN stays null. Do not tap Spaces rows.
Not Visual v1 sign-off.

## Нумерация

После ADR-242 следующий свободный номер — **243**. Не S33.

## Контекст

The Spaces tab was a switcher (`SpaceRow` + select). The boards'
Пространство surface is one context: people, objects, tasks from
SOM `in-space` members. ADR-128 deferred that detail. Goal wave B,
one screen. Do not invent a people cluster or invite chrome.

## Decision

1. **Open.** Tapping a live space selects it (if needed) and opens
   detail. Retap-to-cycle lifecycle yields to this surface.
2. **Members.** Rows are `list_space_members` entities already in
   `selected_entities`, filtered like Search (no space-color /
   lifecycle / relation / signal / notification / schedule).
   `person.contact` is labeled «Человек». Empty is «Нет объектов»,
   store-down is «Нет связи». No «Люди» row.
3. **Chrome.** Header section is «Пространство». Trailing Назад and
   re-tapping Пространства return to the switcher. Member tap opens
   Object View. `compile_v2_public` stays Experimental.

## Consequences

Orb workflow states stay later B. Rollback: restore retap lifecycle
and SpaceRow-only Spaces. Visual v1 still unsigned.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+272 passed, including
`space_member_rows_are_som_members_not_invented_people` and
`space_detail_action_at_finds_member_then_back`.
Panther: shell `a63e1ce9…` pid 1617. Leave Сейчас; do not tap
Spaces rows, Поиск, Inbox, Разрешить. Marker on.
