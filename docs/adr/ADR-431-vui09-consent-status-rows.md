# ADR-431: VUI-09 — consent / pair Status cards read `layout_v2()`

## Статус

Принято, 2026-10-10. Decision Buttons already paint from
`overlay_buttons_v2_source` (ADR-229). Consent capability cards and
the remote-pair client row still used `stacked_row_rect` by hand.
Those Status `DataRow`s now come from the same generated tree as
accept/decline. They invent no actions. PIN stays null. Do not tap
Разрешить or Сопряжь. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-430 следующий свободный номер — **431**. Не S33.

## Контекст

ADR-229 kept capability cards as Status stacked rows so the compiler
would not invent Buttons. Paint still built those rects with
`stacked_row_rect`, a second tree from the decision row.

## Decision

1. **Consent.** `consent.app`, `consent.cap.{i}` or `consent.empty`
   dock on `stacked_row_rect`. Status invents no action.
2. **Remote pair.** `pair.client` docks the same way.
3. **Buttons stay at the bottom.** Accept/decline remain the 300 px
   row. Hits at `y=2250` are unchanged.
4. **Public example.** `consent-public.sui` is the labelled sample.
   Still Experimental.

## Consequences

- Consent/pair cards and Buttons share `layout_v2`. Object View
  overlay geometry is unchanged. Leftover formulas stay Keyboard
  keys, gallery page, lock idle/wake (ADR-232). Rollback: restore
  `stacked_row_rect` in `consent_content_cards`. Still not Visual v1
  sign-off.

## Verification

Host: public `consent.app` is `(x, 430, …, 190)` with no action;
accept/decline still hit `y=2250`. Panther: do not flash; leave
Сейчас.
