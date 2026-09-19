# ADR-153: VUI-07 lock device state — live battery, not a widget

## Статус

Принято, 2026-09-19. No-PIN lock may show a Caption `SemanticText` of
the live fuel-gauge reading. Flashed panther `88121ad2…`: clock `17:50`
and `Заряд 98%`, no Inbox bodies. PIN keypad, deep-idle, and Space
detail are not this slice. Do not save a PIN.

## Нумерация

После ADR-152 следующий свободный номер — **153**. Не S33.

## Контекст

Visual Language 9.6: lock prioritizes time, device state, essential
attention, and unlock. ADR-134 paints time and the tap hint. ADR-148
paints compact attention. Displayd still ignores status-bar commits
while locked, so the status layer's battery is not the lock's battery.
HIA forbids a dashboard of widgets and puts the Orb arc on the
unlocked shell; the lock surface covers that Orb. Remaining chrome
after the shared keyboard is this hole, not another keypad painter.

Callers cannot invent `0%` when `read_battery()` is `None` (host
builds). Color must not mean charge — that collides with attention.

## Decision

1. **`lock_device_view(battery: Option<(percent, charging)>)`**.
   `None` → omit. Else Caption text `Заряд N%` or `Зарядка N%`.
   The type holds percent, charging, and that label — never titles
   or bodies.
2. **`draw_lock_idle` paints the Caption** below the hint, above
   attention, left-aligned with the indicator. No Surface card, no
   determinate track. PIN unlock stays the Field + keypad. Deep-idle
   stays black.
3. **Repaint when the minute, attention key, or battery key
   changes**, never while `sleeping`. Tap is still unlock, not a
   new hit-target.

## Consequences

- Lock can name charge without leaking Inbox content or painting a
  widget grid.
- Host builds stay quiet when `/sys/class/power_supply/maxfg` is
  absent.
- Rollback: drop the Caption argument and restore minute+attention
  refresh.

## Verification

Host: missing reading omits the label; 87% is `Заряд 87%`; charging
is `Зарядка`; neither label contains Inbox text. Panther `88121ad2…`
pid 5391: boot lock without PIN, clock `17:50`, `Заряд 98%`, tap-unlock.
Do not set a PIN. Do not 7-tap.
