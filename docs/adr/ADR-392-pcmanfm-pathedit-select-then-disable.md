# ADR-392: packed PCManFM PathEdit click selects path then disables

## Статус

Принято, 2026-09-21. APP-04 PCManFM chrome on host qemu with a
panther-like seat and host frame clock (ADR-388). Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-391 следующий свободный номер — **392**. Не S33.

## Контекст

ADR-365: PathEdit-band click `400 40` kept FolderView IM
(`166x100`). ADR-391: the same clock lets Filter click `400 760`
map a line caret and type OSK `hi!`.

Same click `400 40` after that clock, with the Filter line-caret
wait:

1. v2 cursor becomes `10x13+644+99` (toolbar PathEdit, not Filter
   y≈752 and not FolderView `166x100`).
2. v2 surrounding jumps from 0 to the current path (37 bytes in
   this tempdir; `selectAll` on PathEdit).
3. v2 disable. Qt logs `hideInputPanel` then `showInputPanel`.
4. No second enable in the 4 s OSK window. OSK does not forward
   `commit_string`.

This is Ctrl+L class (ADR-332: disable, no re-enable), not Filter
class (ADR-391: disable then enable, then type). Do not more Y.
Do not more PathEdit clicks. Do not add a fake `wl_keyboard`.

## Decision

1. **PathEdit tap is not an insert target.** Line caret + path
   surrounding is not typed PathEdit. Filter remains the PCManFM
   field that holds IM after a tap.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 PCManFM PathEdit remaining is Qt chrome losing IM after
  PathEdit focus, not missing compositor v2. Falkon URL flash-
  then-disable (ADR-389) is the same class. Filter stays typed on
  host.
- Rollback: restore the ADR-365 FolderView-only PathEdit asserts.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_pathedit_click_selects_path_then_disables`. dest-no-lock
kept. Do not flash. Leave Сейчас.
