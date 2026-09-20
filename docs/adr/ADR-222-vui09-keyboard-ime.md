# ADR-222: VUI-09 — Keyboard is a Field-bound IME object, swappable with USB

## Статус

Принято, 2026-09-20. On-screen keys are no longer a leftover formula
beside `Field`. `Keyboard` is a privileged composite bound to a
focused Field (Android IME / InputConnection). Its source is
`OnScreen` or `Hardware`. USB HID with `KEY_A` replaces the panel.
No libxkbcommon, no `wl_keyboard`, no new daemon. Volume/power/touch/
haptic nodes stay unopened. PIN stays null. Do not type intent. Not
Visual v1 sign-off.

## Нумерация

После ADR-221 следующий свободный номер — **222**. Не S33.

## Контекст

ADR-221 docked overlay `Field` hits through `layout_v2()` and left
QWERTY/PIN keys as `intent_view` / keypad formula. That split is
wrong for a phone: the keyboard is the input method attached to the
Field, and a USB keyboard must be able to take its place.

Panther cannot use Wayland text-input or libxkbcommon (ADR-012/022/029).
gpio-keys and s2mpg12-power-keys are not a typing keyboard.

## Decision

1. **`Keyboard` object.** `saai-ui-core::Keyboard` binds a Field loc
   and a layout (`Qwerty` / `Pin`). Keystrokes from either source
   apply to that Field's value. `shows_panel()` is true only for
   `OnScreen`.
2. **Vocabulary.** Privileged composite `Keyboard` in `compile_v2()`.
   `compile_v2_public()` rejects it. Compose `layout_v2` docks the
   on-screen reserve only when the component is present. Hardware
   omits it; the Field stays, the panel does not.
3. **Hardware source.** `/proc/bus/input/devices` must show `kbd`,
   `KEY_A`, and a name that is not gpio/power/fts/haptic/volume.
   Shell reads raw `EV_KEY` from that `eventN` only. Enter submits,
   Escape cancels, same as the OSK controls.

## Consequences

- Intent, Wi-Fi password, PIN setup, and lock PIN share one IME
  object. Paint may still draw OSK keys from `intent_view` /
  `pin_keypad_node` while the source is OnScreen. USB unplug returns
  OnScreen. Rollback: restore formula keys and drop `Keyboard`.
  Still not Visual v1 sign-off.

## Verification

Host: bind+apply; USB fixture detected; gpio-keys ignored; compose
without `Keyboard` has no reserve. Panther: flash; Сейчас; do not
open PIN or type intent. USB path is host-injected unless a HID
keyboard is attached.
