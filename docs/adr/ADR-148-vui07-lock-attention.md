# ADR-148: VUI-07 lock essential attention — StatusIndicator, lock-state disclosure

## Статус

Принято, 2026-09-19. No-PIN lock may show a compact `StatusIndicator`
from the live ATTN projection. Flashed panther `55f7bd25…`. No Inbox
bodies, no task titles, no Confirm/Decline. PIN keypad chrome and
Space detail are not this slice. Do not save a PIN.

## Нумерация

После ADR-147 следующий свободный номер — **148**. Не S33.

## Контекст

Visual Language 9.6: lock prioritizes time, device state, essential
attention, and unlock; sensitive content respects lock-state policy.
ADR-134 already paints the clock and tap hint and forbids Inbox
bodies. The remaining essential-attention hole is quiet: Orb/NOW/Inbox
share `has_orb_attention`, but the lock surface does not. HIA forbids a
dashboard of widgets — this is not a widget host.

Policy must withhold fields before paint, not hide already-fetched
text. Tap must not execute. Details stay behind unlock (existing
NOW/Inbox after unlock).

## Decision

1. **`lock_attention_view(store_connected, has_attention)`** takes
   booleans only — never entities, titles, or bodies. Offline →
   `Offline` / «Нет связи». Connected + attention → `Attention` /
   «Требует внимания». Quiet → `None`. Compact `StatusIndicator`;
   `reason` is absent from the type.
2. **`lock_attention_tap(locked)`** is the re-check: while locked the
   only outcome is `UnlockRequired`. It is not a new hit-target — the
   existing tap-to-unlock path stays. After unlock, NOW/Inbox use the
   full projection, not this view.
3. **`draw_lock_idle` paints the indicator** below the hint when
   `Some`. No Surface card. PIN unlock stays dots. Deep-idle stays
   black. Repaint when the minute *or* the attention key changes.

## Consequences

- Lock can name essential attention without leaking Inbox content.
- Callers cannot pass a title into the lock view.
- Rollback: drop the indicator argument and restore minute-only
  refresh.

## Verification

Host: disconnected never carries attention text; connected attention
has empty reason; tap while locked is `UnlockRequired`; attention mark
is Attention color with no Surface card. Panther `55f7bd25…` pid
31354: PIN is currently set, so lock still paints the existing keypad
dots (unchanged). The idle indicator is gated until PIN is absent.
Do not set a PIN.
