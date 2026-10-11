# ADR-458: VUI-09 — public list headers match Inbox stacked top

## Статус

Принято, 2026-10-11. Inbox already locks `ContextHeader` at
`(49, 0, 982, 430)` (ADR-435). Search / Spaces / Me / Wi-Fi /
Bluetooth public examples compiled, but their headings had no
sibling geometry lock. Status invents no action. Public subset
stays Experimental. Leftover named chrome stays closed.
Keyboard keys, gallery page, and lock idle/wake stay formulas.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-457 следующий свободный номер — **458**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-435 docked list / apps headings through `layout_v2`. Inbox has
`layout_v2_inbox_header_matches_stacked_top`. Search gained a
stacked-row hit lock (ADR-457). Spaces / Me / Wi-Fi / Bluetooth
already have row hits. Their public `ContextHeader` slots were
only present, not measured.

Inventing Buttons on those headings, or converting Keyboard keys /
gallery page / lock idle/wake, is not this gate.

## Decision

1. **Host-lock** `search-public.sui`, `spaces-public.sui`,
   `me-public.sui`, `wifi-public.sui`, and `bluetooth-public.sui`
   headings to the Inbox stacked top: `(49, 0, 982, 430)`, no
   action.
2. **Status invents no heading hit.** `540, 250` stays empty.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Primary-tab and list public headings share one Experimental
  geometry. Rollback: drop the sibling header test. Still not
  Visual v1 sign-off.

## Verification

Host: five public list headings match Inbox. Panther: do not
flash; leave Сейчас.
