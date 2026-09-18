# ADR-137: VUI-07 Object View — ObjectSummary

## Статус

Принято, 2026-09-18. Object View paints identity through
`ObjectSummary`. A tap on the NOW summary opens the same HIA-07
screen (Inbox remains an entry). Flashed panther `e2081d84…`:
`vnnnmb` below the clock, `saaios.intent · версия 1`, trailing
`Нет задачи`, `Нет исполнения`. Consent, remote pairing, PIN keypad
chrome, and Space detail are not this slice.

## Нумерация

После ADR-136 следующий свободный номер — **137**. Не S33.

## Контекст

VUI-07 next remaining surface after the developer list. `object_
view_content` already builds `DecisionOverlay` / `AgentSummary`,
then `object_view_details` flattens them to `Vec<String>`, and
`draw_object_view` paints title/status as raw `draw_text`. Section
7.3 already owns identity (title + type/version meta + optional
trailing status) — NOW uses it; Object View does not. Inbox on
panther is empty, so Object View is unreachable there; NOW already
shows the live selected entity as `ObjectSummary` but the comment
still says it is informational only. HIA-07 is an explicit tap, not
an auto-popup. Do not invent an Inbox row. Do not restyle consent,
remote pairing, or PIN keypad chrome. Do not add a Back button in
this slice (zero-action entities stay as they are).

## Decision

1. **`object_view_summary(entity, content) -> ObjectSummary`**. Title
   is the live entity title. Meta is `{entity_type} · версия
   {revision}` (identity, same as NOW). Trailing is
   `StatusIndicator` from `content.state` and `content.status`.
   Related lines and flattened details stay below, not in meta.
2. **`draw_object_view` takes that `ObjectSummary`**. Identity sits
   at `header.y + 140`, below the 120px PIXEL_7 status layer, using
   the same `ObjectSummary` paint NOW uses. Actions unchanged.
3. **NOW summary tap opens Object View** for
   `selected_entities.first()`, same `viewing_entity_id` as an Inbox
   row. Hit rect matches NOW layout. Footer rows stay distinct. No
   auto-popup.

## Consequences

- Object View and NOW share one identity composite.
- Inbox-empty panther can still open a live object from NOW.
- Rollback: restore `draw_object_view(title, state, status)` and
  NOW-informational-only.

## Verification

Host: meta is identity, not the status string; trailing carries the
workflow status; NOW object rect hits and misses the footer.
Panther `e2081d84…` pid 32060: unlock, `Сейчас`, tap the live
summary; Object View shows `vnnnmb` below the clock with
`saaios.intent · версия 1` and `Нет задачи`. Object View actions
were not tapped.
