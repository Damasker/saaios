# ADR-393: packed PCManFM PathEdit disables without OSK

## Статус

Принято, 2026-09-21. APP-04 PCManFM chrome on host qemu with a
panther-like seat and host frame clock (ADR-388). Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-392 следующий свободный номер — **393**. Не S33.

## Контекст

ADR-392: PathEdit click `400 40` maps a line caret and path
surrounding, then v2 disable. OSK does not reach v2. Qt also logged
`commit()` / `hideInputPanel` in that slice — those could have been
the foreign IME, not chrome.

Same click, no OSK, 2 s quiet after the line caret:

1. Line caret `10x13` and path surrounding still appear.
2. v2 disable still fires. No `commit_string`.
3. No second enable.

PathEdit losing IM is PCManFM chrome after PathEdit focus (same
class as Ctrl+L ADR-332), not OSK-induced. Filter still re-enables
and types (ADR-391). Do not more Y. Do not more PathEdit clicks.
Do not add a fake `wl_keyboard`.

## Decision

1. **Do not treat PathEdit as an OSK-vs-steal QLineEdit.** The
   client disables v2 on its own after the tap. Compositor v2 is
   not the remaining PathEdit gap.
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+390. AUTH-10
   stays a later flash-week device pass.

## Consequences

- APP-04 PathEdit remaining is Qt chrome, same class as Falkon URL
  flash-then-disable (ADR-389). Filter stays the typed PCManFM
  field on host.
- Rollback: drop the quiet PathEdit test; keep ADR-392 OSK-miss.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_pathedit_click_disables_without_osk`. dest-no-lock
kept. Do not flash. Leave Сейчас.
