# ADR-365: packed PCManFM PathEdit click without `wl_keyboard` stays FolderView

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-364 следующий свободный номер — **365**. Не S33.

## Контекст

ADR-364: Filter-band `400 760` on a keyboard-less seat keeps
FolderView IM. ADR-327: PathEdit-band `400 40` on a keyboard seat
re-enables v2 without shm paint.

Same helper, one click `400 40` after FolderView enable, OSK
immediately. Not a Y sweep. Not Ctrl+L:

1. Pre-click: v2 enable, IME Activate.
2. `injected click 400 40`.
3. OSK `commit_string`. Still
   `FolderViewListView::inputMethodQuery`. No
   `QLineEdit::inputMethodQuery`. No discard.
4. Main shm unchanged after the click.

Toolbar PathEdit tap does not move IM off the folder list either.
Do not hunt more Y coordinates this week.

## Decision

1. **Do not claim PathEdit typed from click `400 40` on a
   panther-class seat.** IM stays FolderView. Do not add a fake
   `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 PCManFM chrome fields (Filter and PathEdit bands) do not
  take IM from FolderView with a single tap. Probe `QLineEdit`
  still types (ADR-360/362).
- Rollback: drop
  `packed_pcmanfm_pathedit_click_without_seat_keyboard_still_folderview`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_pathedit_click_without_seat_keyboard_still_folderview`.
dest-no-lock kept. Do not flash. Leave Сейчас.
