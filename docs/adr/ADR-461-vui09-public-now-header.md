# ADR-461: VUI-09 — public NOW header is not list stacked top

## Статус

Принято, 2026-10-11. List / overlay headings lock
`(49, 0, 982, 430)` (ADR-458/460). Public NOW already locks tabs,
object, footer, and `now.empty` (ADR-194/199/200/438). Its
`ContextHeader` only named `y=0`. NOW uses `v2_now_header_height`,
then `ObjectSummary`. Status invents no heading hit. Public subset
stays Experimental. Leftover named chrome stays closed. Keyboard
keys, gallery page, and lock idle/wake stay formulas. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-460 следующий свободный номер — **461**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-225 paints NOW heading inside the compiled slot. Flattening
NOW onto the Inbox stacked top would move `ObjectSummary` and
break `open_object`. Inventing a heading Button is not this gate.

## Decision

1. **Host-lock** `now-public.sui` heading to
   `(49, 0, 982, 263)`. `ObjectSummary` docks at `y=263`. The
   rect is not Inbox stacked top.
2. **Status invents no heading hit.** `540, 250` stays empty.
   `open_object` stays on `ObjectSummary`.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public NOW heading stays the NOW formula. Rollback: drop the
  NOW header test. Still not Visual v1 sign-off.

## Verification

Host: public NOW header is `(49, 0, 982, 263)`. Panther: do not
flash; leave Сейчас.
