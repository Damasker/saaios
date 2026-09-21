# ADR-342: Falkon URL KEY_A after second click still does not paint

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-341 следующий свободный номер — **342**. Не S33.

## Контекст

ADR-340: OSK `commit_string` after the URL second-click enable is
not discarded for `m_resetCallback`; Qt returns silently when
`focusObject()` is null. Probe `QLineEdit` with `setFocus` still
types (ADR-336).

Host qemu, same sequential `inject-click 640 20` (ADR-338), then
immediate `inject-key` (evdev KEY_A → xkb 38, wl_keyboard, not IME):

1. Compositor logs `injected synthetic key press+release`.
2. Main toplevel shm hash at that enable equals the hash 700 ms
   after KEY_A (`6cd11128…`). Same no-paint class as OSK (ADR-335).

LocationBar after a synthetic pointer click is not a focused insert
target for IME **or** for seat keyboard.

## Decision

1. **Do not claim the LocationBar shows `a`.** KEY_A is
   protocol-without-paint.
2. **Do not sweep Y.** URL band stays `640 20`.
3. **Do not flash panther.** AUTH-10 stays a later flash-week device
   pass (reboot drops dest-no-lock).

## Consequences

- APP-04 Falkon chrome is enable + OSK forward + KEY_A deliver,
  none of which insert. Probe `QLineEdit` remains the only typed
  Qt 6 field.
- Next compositor key or IME ordering will not paint this bar.
- Rollback: drop the KEY_A falkon_frame test.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
