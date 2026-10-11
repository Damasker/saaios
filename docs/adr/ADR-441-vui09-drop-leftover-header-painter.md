# ADR-441: VUI-09 — drop leftover `paint_context_header`

## Статус

Принято, 2026-10-11. Overlay / compose / list / apps headings already
read the compiled `ContextHeader` slot (ADR-433–435). NOW still
inlined the same 150/2400 paint. The dead `width/20` painter is
gone. Status invents no action. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off.

## Нумерация

После ADR-440 следующий свободный номер — **441**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-433/434/435 rolled back to `paint_context_header(content)`. That
function used `content.width / 20` and never looked up the compiled
slot. After those slices it had no live caller (`#[allow(dead_code)]`).
NOW still copied the slot inset by hand.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover heading painter was not on that
list.

## Decision

1. **One painter.** `draw_now` calls `paint_overlay_context_header`
   with `chrome.header`. Overlay, compose, list, apps, and NOW share
   that helper.
2. **Delete** `paint_context_header`. No `width/20` heading tree.
3. **Not this slice.** `Frame::Root` / `draw_root` stay the test
   fallback (ADR-165). `task_confirm_view` stays unused. Keyboard
   keys, gallery page, and lock idle/wake stay formulas.

## Consequences

- Named `ContextHeader` paint has one origin. Rollback: restore
  `paint_context_header` and the NOW inline. Still not Visual v1
  sign-off.

## Verification

Host: `paint_context_header` is gone; `draw_now` calls the slot
helper. Panther: do not flash; leave Сейчас.
