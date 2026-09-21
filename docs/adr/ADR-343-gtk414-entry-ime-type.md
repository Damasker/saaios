# ADR-343: OSK IME types into a packed musl GTK 4.14.4 Entry

## Статус

Принято, 2026-09-21. APP-04 GTK 4.14 toolkit on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-342 следующий свободный номер — **343**. Не S33.

## Контекст

ADR-325: host glibc GTK 4.18 Entry with `grab_focus` types OSK `hi!`.
Panther runs Alpine 4.14.4 (ADR-310/320). 4.18 typing is a different
toolkit. Packed Qt 5.15/6.6 `QLineEdit` with `setFocus` already types
(ADR-330/336). The panther GTK Entry was unproven.

Host qemu, `qemu-aarch64-static -L /tmp/gtk4-probe`, binary
`gtk414-entry` (same 4.14.4 headers/libs as `gtk4-demo`):

1. IME client bound first.
2. Focused `GtkEntry` → `text-input-v3 enable`, IME `Activate`.
3. Same Keyboard actions as ADR-325 (`h i !` through IME).
4. `notify::text` prints `GTK_ENTRY_TEXT=hi!`.

## Decision

1. **APP-04 packed GTK 4.14 path types.** OSK IME inserts `hi!` into
   a real musl 4.14.4 Entry on host `saai-displayd`.
2. **Do not claim a panther field was typed.** Touch-only seat and
   chrome PathEdit/LocationBar stay protocol-without-paint.
3. **Do not flash panther.** Next displayd flash must still carry
   ADR-311 bounds + ADR-319 v2 + ADR-328 delete + ADR-339 queue.

## Consequences

- Host GTK 4.14 typing is compositor-true, same class as 4.18 and the
  Qt probes. Chrome without `grab_focus` is still untyped.
- Rollback: drop `gtk414_entry.c` / the alpine gtk4_ime test.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. Probe binary
`/tmp/gtk4-probe/bin/gtk414-entry` (not committed). dest-no-lock kept.
Do not flash. Leave Сейчас.
