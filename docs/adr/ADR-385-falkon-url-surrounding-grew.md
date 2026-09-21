# ADR-385: packed Falkon URL OSK grows v2 surrounding, not shm

## Статус

Принято, 2026-09-21. APP-04 Falkon chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-384 следующий свободный номер — **385**. Не S33.

## Контекст

ADR-384: URL click `640 20` then immediate OSK forwards v2
`commit_string`; shm stays `6cd11128…`. Silent `focusObject`
null vs applied-without-paint was unproven. v2
`SetSurroundingText` / `SetCursorRectangle` had no compositor
log. Waiting for IME `Activate` after enable let Falkon
`disable` win the race.

Host `saai-displayd` now prints v2 `surrounding` byte length
(not the text) and `cursor` size. Same keyboard-less seat, one
click `640 20`, OSK on the first `text-input-v2 enable` (do
not wait for IME Activate):

1. Surrounding after OSK is greater than at enable. Qt applied
   the commit into an IM widget.
2. Zero toplevel shm commits. Main shm stays `6cd11128…`.

Same class as gtk4-demo (ADR-378): IM apply, no mapped buffer.
Packed musl Qt 6 QLineEdit still types. Do not claim typed
LocationBar. Do not more Y. Do not rustfmt overlay
`text_ime.rs`. Next displayd flash also carries these logs.
Do not log surrounding text.

## Decision

1. **Do not treat Falkon URL OSK as a missing IME apply.**
   Surrounding grew. Toplevel shm did not. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 Falkon URL remaining is LocationBar paint after Qt
  applied the commit, not a null `focusObject` at apply time.
  Rollback: drop the v2 surrounding/cursor printlns and the
  grow assert on
  `packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.
dest-no-lock kept. Do not flash. Leave Сейчас.
