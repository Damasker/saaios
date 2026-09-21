# ADR-387: packed PCManFM FolderView OSK surrounding stays 0

## Статус

Принято, 2026-09-21. APP-04 PCManFM chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-386 следующий свободный номер — **387**. Не S33.

## Контекст

ADR-363: OSK into keyboard-less PCManFM v2 hits
`Fm::FolderViewListView` (no `inputMethodQuery`), not Filter.
ADR-385: Falkon URL OSK grows surrounding `0 → 3` (`hi!`).
Whether FolderView applies that commit was unproven.

Same keyboard-less seat, no click, OSK after FolderView enable:

1. `text-input-v2 surrounding bytes=0` before and after OSK.
   Cursor rectangle is `166x100` (the list view), not a line
   caret.
2. `commit_string` is forwarded. No `discard commit_string`.
3. Main shm stays `484823fc…`. Still FolderViewListView.

Qt did not apply OSK into a text field. That is not the Falkon
URL class (applied, no paint). PathEdit/Filter remain untyped.
Do not more Y. Packed musl Qt 5 QLineEdit still types.

## Decision

1. **Do not treat FolderView enable as a typed Filter/PathEdit.**
   Surrounding stays empty. Do not add a fake `wl_keyboard`
   (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 PCManFM remaining is a focused `QLineEdit`
  (Filter/PathEdit), not an apply-without-paint. Rollback: drop
  the surrounding-stays-0 assert on
  `packed_pcmanfm_osk_without_seat_keyboard_hits_folderview_not_filter`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_osk_without_seat_keyboard_hits_folderview_not_filter`.
dest-no-lock kept. Do not flash. Leave Сейчас.
