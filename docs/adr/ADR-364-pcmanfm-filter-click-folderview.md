# ADR-364: packed PCManFM Filter click without `wl_keyboard` stays FolderView

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-363 следующий свободный номер — **364**. Не S33.

## Контекст

ADR-363: OSK into Activated enable hits `Fm::FolderViewListView`,
not Filter. ADR-331: Filter-band click `400 760` on a keyboard
seat re-enables v2 without shm paint. ADR-360: tap-to-focus works
on a real `QLineEdit`.

Same packed PCManFM, `ShowFilter=true`, `SAAIOS_SEAT_NO_KEYBOARD=1`,
one click `400 760` after FolderView enable, OSK immediately.
Not a Y sweep. Not Ctrl+L. Not Falkon:

1. Pre-click: v2 enable, IME Activate, no `keyboard focus set`.
2. `injected click 400 760`.
3. OSK `commit_string`. No `QLineEdit::inputMethodQuery`.
   Still `FolderViewListView::inputMethodQuery`. No discard.
4. Main shm after the click equals shm at enable (`484823fc…`
   class).

A Filter-band tap does not move IM from the folder list to Filter.
Do not start a Y sweep to hunt the bar.

## Decision

1. **Do not claim Filter typed from click `400 760` on a
   panther-class seat.** IM stays FolderView. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 PCManFM remaining insert still needs a focused Filter
  `QObject`. One bottom-band tap is not that. Falkon URL remains
  `focusObject` null (ADR-340).
- Rollback: drop
  `packed_pcmanfm_filter_click_without_seat_keyboard_still_folderview`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_filter_click_without_seat_keyboard_still_folderview`.
dest-no-lock kept. Do not flash. Leave Сейчас.
