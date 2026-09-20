# ADR-242: Object View shows existing facts, links, OAM

## Статус

Принято, 2026-09-20. Object View content is chrome; panther flash
followed host-green. PIN stays null. Not Visual v1 sign-off.

## Нумерация

После ADR-241 следующий свободный номер — **242**. Не S33.

## Контекст

Object View already maps workflow types and OAM unavailability.
Related still kept only the first outgoing link and treated a Space
id as a peer. Unknown types dumped identifier keys as facts. The
boards want one object: facts, related, actions — existing SOM
links and OAM that exist, not AWS/kubectl, not invented people.

## Decision

1. **Related.** Every live entity–entity SOM link whose peer is in
   the selected space, outgoing and incoming. Dangling refs and
   `saaios.in-space` stay omitted. Inferred unconfirmed links stay
   marked «Возможно связано».
2. **Facts.** Unknown types show string/number/bool properties.
   Keys `id` / `*_id` / `*_ids`, UUID strings, null, arrays, and
   objects are not facts. Empty stays «Нет дополнительных данных».
3. **OAM.** Unavailable/deny stays a blocked pattern. No invented
   action buttons. Confirm and workflow follow-paths stay as they
   are.

## Consequences

Space detail (SOM members) and Orb workflow states stay later B
slices. Rollback: first outgoing related + raw property dump.
Visual v1 still unsigned.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+270 passed, including
`object_view_content_lists_every_existing_related_peer`,
`object_view_content_omits_dangling_and_space_membership`,
`object_view_content_omits_identifier_properties`.
Panther: shell `bfba710f…` pid 1496. Leave Сейчас; do not tap
Поиск, Inbox, Spaces, Object View rows. Marker on.
