# ADR-464: VUI-09 — public summaries dock on `stacked_row_rect`

## Статус

Принято, 2026-10-11. `IntentSummary`, `TaskSummary`, and
`AgentSummary` are Experimental public gallery names. `layout_v2`
dropped them as leftover Fill leaves. They now occupy stacked
slots. Status invents no action. Button a11y invents none.
`object-public.sui` stays header / `ObjectSummary` / fact
`DataRow` / permission. Public subset stays Experimental.
Leftover named chrome stays closed. Keyboard keys, gallery page,
and lock idle/wake stay formulas. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-463 следующий свободный номер — **464**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-05 named those composites on Object View. Live Object View
facts stay `DataRow` (ADR-429). A third-party document that names
the summaries compiled, then lost stacked geometry. Inventing
`open_object` on them would disagree with Status Object View.

## Decision

1. **Dock** `IntentSummary` / `TaskSummary` / `AgentSummary` on
   `stacked_row_rect` through `layout_v2`.
2. **Status invents no action.** A Button a11y still invents none.
   `object-public.sui` does not name them.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public summaries share list geometry. Rollback: drop them from
  the stacked match. Still not Visual v1 sign-off.

## Verification

Host: three Status summaries match stacked rows; Button invents
no hit. Panther: do not flash; leave Сейчас.
