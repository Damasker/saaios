# ADR-395: PathEdit still disables after no-steal Activated

## Статус

Принято, 2026-09-21. APP-04 PCManFM chrome on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do
not flash displayd this week.

## Нумерация

После ADR-394 следующий свободный номер — **395**. Не S33.

## Контекст

ADR-394: a second xdg_toplevel no longer steals Activated.
QCompleter types. Hypothesis: PathEdit disable (ADR-392/393) was
that steal (completer QListView).

Same PathEdit click `400 40`, no OSK, 2 s quiet, after ADR-394:

1. Line caret and path surrounding still appear.
2. v2 disable still fires. No second enable.

PathEdit losing IM is not Activated-steal from a second toplevel.
Do not more PathEdit clicks. Do not more Y.

## Decision

1. **Do not treat ADR-394 as typed PathEdit.** Filter remains the
   PCManFM field that holds IM after a tap (ADR-391).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- APP-04 PathEdit remaining is still Qt chrome after PathEdit
  focus. Falkon URL flash-then-disable is unproven against ADR-394
  without another URL click sequence.
- Rollback: none; ADR-393 test still holds.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_pathedit_click_disables_without_osk`. dest-no-lock
kept. Do not flash. Leave Сейчас.
