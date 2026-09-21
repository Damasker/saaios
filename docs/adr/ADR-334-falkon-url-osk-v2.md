# ADR-334: OSK hi! reaches packed Falkon URL v2

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-333 следующий свободный номер — **334**. Не S33.

## Контекст

ADR-333: `inject-click 640 20` into windowed packed Falkon (single tab
hidden) enables v2; a single IME `commit_string("hi!")` in that window
is forwarded. ADR-328/330: the same OSK keystroke sequence types `hi!`
into a focused probe `QLineEdit`. Packed PCManFM after a click gets
that sequence without a new shm (ADR-329).

Same URL enable window, OSK actions `h i toggle backspace i !`:

1. `text-input-v2 commit_string` ×4
2. `text-input-v2 delete_surrounding` ×1

Commit in the enable window. Waiting for click settle still lets Qt
disable first (ADR-333).

## Decision

1. **Do not claim the LocationBar shows `hi!`.** Protocol only: OSK
   IME reaches the live v2 field. No widget text dump, no new shm
   claim.
2. **Do not sweep Y.** URL band remains `640 20` (ADR-333).
3. **Do not flash panther.** URL chrome stays off screen on the
   phone (ADR-317).

## Consequences

- APP-04 host OSK insert into Falkon chrome is compositor-true and
  still unproven paint. Probe QLineEdit (ADR-330) remains the only
  typed Qt field.
- Rollback: drop `falkon_url_osk_hi_bang_reaches_v2`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
