# ADR-472: VUI-09 — Object related is one Button per SOM peer

## Статус

Принято, 2026-10-11. Designer handoff
`graphic-kit/handoff-vui09` (object-default: two chevron rows).
ADR-471 made a single `object.related` Button. Live SOM can resolve
more than one peer. Each resolvable peer is `object.related.{i}`.
Zero peers hide the block. Workflow path captions stay Status.
Quick actions stay the existing real `actions` list. Public subset
stays Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-471 следующий свободный номер — **472**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The handoff forbids a joined related caption and invented
«Open Settings» labels. `related_object_lines` already lists every
live peer. One Button for the first peer hid the rest.

## Decision

1. **One Button per peer.** `object.related.{i}` docks below
   `ObjectSummary` in peer order. Tap follows that peer id.
   Unresolved peers stay omitted.
2. **Public default is two rows.** `object-public.sui` names
   `object.related.0` and `object.related.1`. Status related still
   invents no action.
3. **Workflow captions are not peers.** Intent/task path text stays
   a Status fact when no SOM peer list exists.
4. **Not this slice.** Do not invent OAM glyphs. Do not add section
   headers. Do not promote Stable. Do not check Acceptance.

## Consequences

- Related matches the handoff list. Rollback: single
  `object.related` Button. Still not Visual v1 sign-off.

## Verification

Host: two public related Buttons; live index follow. Panther: do
not flash; leave Сейчас.
