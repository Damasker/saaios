# ADR-163: VUI-07 interrupted key — same target on down and up

## Статус

Принято, 2026-09-19. Intent / Wi-Fi / PIN keyboards commit an action
only when the finger went down and came up on the same action.
Pressed highlight still follows the live finger (ADR-151). Do not
type a PSK. Do not send. Do not type PIN digits. Leave Отмена.

## Нумерация

После ADR-162 следующий свободный номер — **163**. Не S33.

## Контекст

Lock already refuses a drag that starts on the surface and ends
elsewhere (unlock on release of a still-over-lock touch). Tabs refuse
a drag through the bar. Keyboard `up()` used only `last_touch_pos`,
so a slide from the Field onto Отправить would send, and a slide
from one key onto another would type the second. Component library
Input matrix: interrupted drag.

## Decision

1. **`committed_action(down, up)`** returns the action only when both
   sides name the same string. Otherwise the gesture is cancelled.
2. **Intent, Wi-Fi password, PIN setup, and lock PIN keypad** store
   the down action and gate `up()` through that helper. Visual
   `pressed_key` is unchanged.
3. **No-PIN lock unlock** is unchanged: any release still unlocks.

## Consequences

- Slide off a key, or onto a different key/control, types nothing.
- Отмена still works: down and up on the same control.
- Rollback: fire `intent_action_at(last_touch_pos)` on every up.

## Verification

Host: Q→Q commits; Field→Q, Q→W, Q→off, Отмена→Отправить do not;
Отмена→Отмена does. Panther: NOW → Новое намерение, screenshot,
Отмена. Do not type. Do not send.
