# ADR-390: panther idle frame clock when no flip is pending

## Статус

Принято, 2026-09-21. APP-04 compositor. Host gtk4-demo OSK still
paints (ADR-388). Panther idle path compiles (`--features
panther-hardware`). Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-389 следующий свободный номер — **390**. Не S33.

## Контекст

ADR-388: host has no DRM VBlank; a 16 ms timer acks
`wl_surface.frame` so gtk4-demo can commit after IME. Panther still
acked frames only after present VBlank. After the last present,
VBlank stops. IME apply without a new buffer then requests a frame
callback that never fires — the same deadlock gtk4-demo had on host.

Ack-on-commit spun saai-shell. Do not do that. While a flip is
in-flight, VBlank still acks shown surfaces. Idle (`!flip_pending`)
needs the same refresh clock as host.

## Decision

1. **`send_pending_frames` is shared.** Host always runs it at 16 ms
   (`host frame clock`). Panther skips the tick while
   `flip_pending`, then runs it (`idle frame clock`).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385 **and ADR-390**.
   AUTH-10 stays a later flash-week device pass. Do not add a fake
   `wl_keyboard` (ADR-012).

## Consequences

- Packed gtk4-demo search_entry OSK still commits shm on host.
- Panther gtk4-demo OSK paint stays unproven until that flash.
- Falkon URL disable-before-OSK (ADR-389) is unchanged.
- Rollback: keep the host-only timer; drop the panther idle tick.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_search_entry_osk_commits_shm`. Host `cargo check
-p saai-displayd --features panther-hardware`. dest-no-lock kept.
Do not flash. Leave Сейчас.
