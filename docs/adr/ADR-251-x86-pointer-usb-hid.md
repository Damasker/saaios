# ADR-251: x86 displayd pointer + USB HID keyboard

## Статус

Принято, 2026-09-20. Host displayd. Not Visual v1 sign-off.
PIN stays null. Panther chrome unchanged this slice.

## Нумерация

После ADR-250 следующий свободный номер — **251**. Не S33.

## Контекст

Windowed x86 displayd (ADR-250) still had a phone seat: touch on
panther, synthetic keyboard on the host, no `wl_pointer`. A laptop
surface needs pointer and a USB HID typing keyboard. Volume, power,
touchscreen, and haptics are not HID typing devices and stay out of
this seat (wave E on panther). Pixel voice stays blocked (ADR-092).

## Decision

1. **x86 seat:** `wl_pointer` + `wl_keyboard`. No touch.
2. **Panther seat unchanged:** touch only (ADR-012). No pointer, no
   HID keyboard capability.
3. **USB HID classification** from `/proc/bus/input/devices`: keep
   devices that type `KEY_A` (keyboard) or advertise `mouse`
   (pointer). Drop gpio/power/volume/touch/haptic names.
4. **Not wave E.** This does not open volume/power/modem/cameras.

## Consequences

Headless inject-key tests still use the existing keyboard handle.
Next D slice is logical layout for the windowed shell, not panther
chrome. Rollback: revert `saai-displayd` hid + pointer.

## Verification

Host: `cargo test -p saai-displayd --offline --bin saai-displayd --
usb_hid_keyboard_and_pointer_are_kept_volume_power_touch_are_not
x86_seat_has_pointer_and_keyboard_panther_stays_touch`.
Panther: no flash. Marker on. Leave Сейчас.
