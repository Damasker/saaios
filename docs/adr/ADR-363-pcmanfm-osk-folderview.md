# ADR-363: packed PCManFM OSK without `wl_keyboard` hits FolderView, not Filter

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-362 следующий свободный номер — **363**. Не S33.

## Контекст

ADR-357: packed PCManFM on `SAAIOS_SEAT_NO_KEYBOARD=1` enables v2
and keeps it. ADR-362: a focused `QLineEdit` types OSK in the
enable window. Hypothesis: ShowFilter Filter is that field.

Same package, `ShowFilter=true`, no click, no Ctrl+L, OSK `hi!`
immediately on enable:

1. First shm `58f45fc6…` then settle `484823fc…` **before** enable
   (window paint, not insert).
2. `text-input-v2 enable` + IME Activate. No `keyboard focus set`.
3. Qt: `Fm::FolderViewListView::inputMethodQuery` missing (×3).
   No `discard commit_string`.
4. OSK `commit_string` ×4 + `delete_surrounding`. Main shm stays
   `484823fc…`.

Durable v2 enable is the folder list, not Filter `QLineEdit`.
FolderView does not implement `inputMethodQuery`. That is why
Filter is not typed without a click, and why protocol-without-paint
survives Activated.

## Decision

1. **Do not claim Filter typed from Activated enable.** The IM
   object is `FolderViewListView`. Do not add a fake `wl_keyboard`
   (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 PCManFM remaining insert is a focused Filter/PathEdit
  `QObject`, not compositor enable. Falkon URL is still
  `focusObject` null (ADR-340).
- Rollback: drop
  `packed_pcmanfm_osk_without_seat_keyboard_hits_folderview_not_filter`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_osk_without_seat_keyboard_hits_folderview_not_filter`.
dest-no-lock kept. Do not flash. Leave Сейчас.
