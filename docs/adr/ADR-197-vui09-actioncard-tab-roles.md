# ADR-197: VUI-09 — map ActionCard and tab labels onto TextRole

## Статус

Принято, 2026-09-20. HEAD shell is `5eb6a27f…`. ActionCard title/status/button
and tab labels use `Label` / `Caption` through `fonts.resolve` / `role_px`. They do
not map onto Title (72) or Body (48). Selected vs idle tabs keep
weight and color, not a second size. Badge, status-bar time, keys,
and gallery leftovers stay named. `compile()` stays v1. No new
daemon. Do not 7-tap. Leave Сейчас.

## Нумерация

После ADR-196 следующий свободный номер — **197**. Не S33.

## Контекст

ADR-188 named leftover draw sizes so no call site kept a raw `38.0`.
ADR-193 left mapping ActionCard 38/27/25 and tab 31/27 onto a
`TextRole` as Visual v2 item 3. Title is 72 physical — that would
blow up list rows. Label is 42 (semibold 14) and Caption is 36
(regular 12). ActionCard already used semibold for the name and
regular for the status. Tab selection already uses Elevated / Accent
/ semibold (ADR-168); size must not encode selected.

## Decision

1. **ActionCard** title and in-card button: `TextRole::Label`.
   Status: `TextRole::Caption`.
2. **Tab labels** (selected and idle): `role_px(TextRole::Caption)`.
   Selected stays semibold; idle stays regular.
3. **Do not retoken** `TAB_BADGE_PX`, status time/battery, keys,
   Orb menu, gallery, or app-tile leftovers.
4. **Flash** this paint. Hits stay the v1 tab strip.

## Consequences

- List rows and the four tabs speak the same type scale as Object
  Summary captions. Rollback: restore the five leftover constants.
  Next: remaining named leftovers, or Experimental→Stable after
  Visual v1. Still not Visual v1 sign-off.

## Verification

Host: `action_card_and_tabs_use_text_roles`; no Title in those
paths. Panther: Сейчас tab labels Caption-sized; Inbox rows Label/
Caption; leave Сейчас.
