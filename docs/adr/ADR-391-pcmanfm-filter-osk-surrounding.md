# ADR-391: packed PCManFM Filter click OSK grows surrounding

## Статус

Принято, 2026-09-21. APP-04 PCManFM chrome on host qemu with a
panther-like seat and host frame clock (ADR-388). Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-390 следующий свободный номер — **391**. Не S33.

## Контекст

ADR-364: Filter-band click `400 760` kept FolderView IM
(`166x100`). ADR-387: FolderView OSK surrounding stays 0.
ADR-388: host frame clock lets clients commit after IME.

Same click `400 760` after that clock:

1. v2 cursor becomes `10x13+200+99`, then `10x13+334+752`
   (Filter bar, not the folder list).
2. v2 disable then enable (focus steal, then Filter holds IM).
3. OSK after the line caret: surrounding `0 → 3` (`hi!`).
4. Toplevel shm changes (`54b10dfc…` → `09272bfd…`).

Packed Qt 5 QLineEdit already types. This is the PCManFM Filter
field on the panther-class seat. PathEdit click `400 40` still
FolderView (ADR-365). Do not more Y. Do not add a fake
`wl_keyboard`.

## Decision

1. **Filter tap is an insert target once frames flow.** Do not treat
   FolderView enable as typed Filter.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 PCManFM Filter remaining on panther is that flash, not a
  missing compositor v2. PathEdit and Falkon URL remain untyped.
- Rollback: restore the ADR-364 hash-eq / FolderView-only asserts.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_filter_click_osk_grows_surrounding`. dest-no-lock
kept. Do not flash. Leave Сейчас.
