# ADR-471: VUI-09 — Object related row is a concept Button

## Статус

Принято, 2026-10-11. ADR-470 reopened board chrome. The Object
board and `surface-object.jpg` show related items as tappable rows,
not a Status caption. When a related peer exists, `object.related`
is a `DataRow` with `a11y = Button`. Status related still invents
no action. First live peer follow is that Button. Public subset
stays Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-470 следующий свободный номер — **471**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-429 docked `object.related` as Status so a third-party overlay
could name a fact. The concept related list has a chevron. Live
SOM peers already exist (`related_object_lines`); the row just
did not hit.

Inventing «Поделиться» / AWS buttons is not this slice. Quick
actions stay the existing real `actions` list.

## Decision

1. **Public.** `object-public.sui` names `object.related` as Button.
   It docks below `ObjectSummary` at the same fact slot. Action is
   interned `loc`. Status related invents none.
2. **Live.** `object_overlay_v2_source` emits that loc as Button
   when a related line exists. A hit follows the first live peer.
   Missing peers stay omitted.
3. **Not this slice.** Do not invent extra quick-action glyphs.
   Do not promote Experimental → Stable. Do not check Acceptance.
   Keyboard keys, gallery page, and lock idle/wake stay formulas.

## Consequences

- Related matches the concept list row. Rollback: Status related
  again. Still not Visual v1 sign-off.

## Verification

Host: public related Button hits interned `loc`; live first peer
follow. Panther: do not flash; leave Сейчас.
