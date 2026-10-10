# ADR-427: VUI-09 — empty Object View header reads `layout_v2()`

## Статус

Принято, 2026-10-10. Named chrome already paints overlays from
`overlay_buttons_v2_source` when actions exist (ADR-229). A
zero-action Object View still built a procedural `object_view`
header leaf. That header now comes from the same generated
`layout_v2()` tree. No invented Buttons. Public
`object-public.sui` compiles through `compile_v2_public()` and
lays out without `saai-shell` internals. Still Experimental.
PIN stays null. Do not open Object View this slice. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-425 следующий свободный номер на `pixel-7-main` — **426**,
но **426** уже занят на `wip/hdi-lab`. Этот срез берёт **427**.
Не S33.

## Контекст

ADR-229 decision 3 kept zero-action Object View on a hand-built
header leaf so the compiler would not invent Buttons. `layout_v2`
only entered `v2_decision_node` when overlay Buttons were present,
so a header-only overlay compiled as a fill. That was a second
tree for the same overlay family.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. Empty Object View was not on that list.

## Decision

1. **Header-only overlay.** A screen with `ContextHeader`, no named
   tabs, and no stacked/grid/Field rows docks through
   `v2_decision_node` even when the button list is empty.
2. **No invented Buttons.** `v2_decision_node` omits the button row
   when there are no Button locs. Hits invent none.
3. **Live paint.** Zero-action and N-action Object View both call
   `overlay_decision_paint("object", …)`. The procedural
   `object_view()` layout is gone.
4. **Public example.** `docs/os/ui/examples/object-public.sui` is
   the labelled Object View sample: `ContextHeader` plus Status
   `ObjectSummary`. No tabs. Status invents no `open_object`.
   `compile_v2_public()` accepts it. The label stays Experimental
   (ADR-185/265).

## Consequences

- Empty Object View header and hit-test share `layout_v2`.
- Public examples now include Object View and all lay out through
  `compile_v2_public` + `layout_v2` without shell internals.
- Leftover formulas stay Keyboard keys, gallery page, lock
  idle/wake. Thin tuning stays last (ADR-213). Rollback: restore
  `object_view(width, height, 0)` in the Frame builder. Still not
  Visual v1 sign-off.

## Verification

Host: empty object overlay header fills the panel; two-action
header shrinks above the 300 px row; `object-public.sui` compiles
public and lays out a `ContextHeader`; every public example lays
out without `saai-shell`. Panther: do not flash; leave Сейчас.
