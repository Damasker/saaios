# ADR-398: Falkon URL still flash-disables after no-steal

## Статус

Принято, 2026-09-21. APP-04 Falkon chrome on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do
not flash displayd this week.

## Нумерация

После ADR-397 следующий свободный номер — **398**. Не S33.

## Контекст

ADR-394: a second xdg_toplevel no longer steals Activated.
QCompleter types. Hypothesis: Falkon URL flash-then-disable
(ADR-389) was that steal (dropdown / WebEngine window).

Same existing URL click `640 20`, OSK on first enable, after
ADR-394:

1. Focus-flash shm `f0e21a69…` still happens.
2. v2 disable still fires before OSK `commit_string`.
3. Toplevel returns to `6cd11128…`. Surrounding stays 0.

Falkon LocationBar remaining is not Activated-steal from a second
toplevel. Same class as PathEdit after ADR-395. Do not more Falkon
click sequences. Do not more Y.

## Decision

1. **Do not treat ADR-394 as typed LocationBar.** URL chrome still
   disables before OSK.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- APP-04 Falkon remaining is still Qt chrome after URL focus-flash.
  Filter and gtk4-demo fields stay typed on host.
- Rollback: none; ADR-389 asserts still hold.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard`.
dest-no-lock kept. Do not flash. Leave Сейчас.
