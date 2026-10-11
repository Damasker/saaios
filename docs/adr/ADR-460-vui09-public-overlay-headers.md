# ADR-460: VUI-09 — public overlay headers match Inbox stacked top

## Статус

Принято, 2026-10-11. List headings already lock
`(49, 0, 982, 430)` (ADR-435/458). Consent / remote-pair public
examples compiled and locked Status cards (ADR-431/432), but their
headings only named `y=0` / `h=430`. Status invents no action.
Fingerprint wrap stays paint. Public subset stays Experimental.
Leftover named chrome stays closed. Keyboard keys, gallery page,
and lock idle/wake stay formulas. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-459 следующий свободный номер — **460**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-458 locked Search / Spaces / Me / Wi-Fi / Bluetooth headings
to the Inbox stacked top. Consent and remote-pair use the same
stacked `ContextHeader` slot, then Status `DataRow`s and decision
`Button`s. Inventing a heading Button, or flattening fingerprint
into `ActionCard`, is not this gate.

## Decision

1. **Host-lock** `consent-public.sui` and
   `remote-pair-public.sui` headings to the Inbox stacked top:
   `(49, 0, 982, 430)`, no action. `540, 250` stays empty.
2. **Status invents no heading hit.** Accept / decline stay the
   only overlay Buttons. Fingerprint stays Status.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public overlay headings share list geometry. Rollback: drop the
  overlay header test. Still not Visual v1 sign-off.

## Verification

Host: consent / remote-pair headings match Inbox. Panther: do not
flash; leave Сейчас.
