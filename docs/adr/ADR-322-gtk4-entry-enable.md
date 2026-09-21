# ADR-322: host GTK 4.18 `Entry` sends `zwp_text_input_v3::enable`

## Статус

Принято, 2026-09-21. APP-04 GTK toolkit half on host. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-321 следующий свободный номер — **322**. Не S33.

## Контекст

Host OSK tests (ADR-270) used a dedicated v3 client that called
`enable()` itself. Qt on panther never binds v3 (ADR-317/318/319).
GTK is the v3 toolkit. Host GTK 4.18 already framed (ADR-305) with a
`Label`. A focused `Entry` was unproven. Alpine 4.14.4 `gtk4-demo
--run=entry` is not an Entry: it maps the default demo window (same
shm hash as unknown `--run` names) and never `enable`s. The Alpine
sysroot has no GTK headers, so a 4.14 Entry binary was not compiled
this slice.

## Decision

1. **`gtk4_hello.py` presents a focused `Gtk.Entry`.** displayd logs
   `text-input-v3 enable` and `text-input-v2 enable` when those
   requests arrive.
2. **Host GTK 4.18 does `enable`.** `gtk4_commits_an_shm_frame_on_host_displayd`
   requires the v3 line. Seat on host has keyboard+pointer.
3. **Do not call Alpine `gtk4-demo --run=entry` an Entry.** 4.14
   Entry-on-touch-only-seat stays unproven until a real 4.14 field
   exists. Next displayd flash still carries ADR-311 bounds + ADR-319
   v2.

## Consequences

- APP-04 GTK host path is a real widget, not only the dedicated
  client. Panther OSK still waits that flash (Qt v2) and a focused
  field.
- Rollback: restore the Label-only probe; drop the enable println.

## Verification

Host `cargo test -p saai-displayd --test gtk4_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
