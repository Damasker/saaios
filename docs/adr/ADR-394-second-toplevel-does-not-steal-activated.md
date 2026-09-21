# ADR-394: second xdg_toplevel does not steal Activated

## Статус

Принято, 2026-09-21. APP-04 compositor on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash displayd
this week.

## Нумерация

После ADR-393 следующий свободный номер — **394**. Не S33.

## Контекст

ADR-352: mapping a toplevel sets `xdg_toplevel` Activated. QCompleter
maps a second `xdg_toplevel` (QListView), not `xdg_popup`. That second
map unset Activated on the field. Qt then `text-input-v2 disable`.
OSK missed. Same class as PathEdit (ADR-392/393) and Falkon URL
flash-then-disable (ADR-389).

Empty `new_popup` still left real xdg_popup clients unconfigured.
QCompleter does not hit that path.

## Decision

1. **First mapped toplevel keeps Activated.** A later toplevel is
   recorded in `focus_history` and configured, but does not steal
   the seat (`mapped toplevel without steal`). Destroy still restores
   the previous live toplevel.
2. **`new_popup` sends configure** and the frame clock acks popup
   surfaces. Unused by this QCompleter probe.
3. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390 **and
   ADR-394**. AUTH-10 stays a later flash-week device pass.

## Consequences

- Host Qt5 QLineEdit + QCompleter types OSK `hi!` with two toplevels.
- gtk4-demo search_entry OSK still paints.
- PathEdit / Falkon URL remain unproven until their own tests. Do
  not more PathEdit or Falkon click sequences as the next default.
- Rollback: activate every newly mapped toplevel (ADR-352 first map).

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
osk_ime_types_hi_bang_into_qt5_lineedit_with_completer_popup`. Host
`cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_commits_shm`. Host `cargo check
-p saai-displayd --features panther-hardware`. dest-no-lock kept.
Do not flash. Leave Сейчас.
