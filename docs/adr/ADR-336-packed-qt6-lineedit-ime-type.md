# ADR-336: OSK IME types into a packed musl Qt 6.6.3 QLineEdit

## Статус

Принято, 2026-09-21. APP-04 Qt6 toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-335 следующий свободный номер — **336**. Не S33.

## Контекст

ADR-330: packed musl Qt 5.15.10 `QLineEdit` with `setFocus` receives
OSK `hi!`. ADR-334/335: packed Falkon (Qt 6.6.3) URL click enables v2
and OSK reaches that field; main shm does not change.

Packed probe is the same `tests/qt5_lineedit.cpp`, linked with zig
`aarch64-linux-musl` against the Falkon package Qt 6.6.3 and Alpine
`qt6-qtbase-dev` 6.6.3-r1 headers. Run under `qemu-aarch64-static -L`
that package (`PACKED_QT6_LINEEDIT`, `FALKON_PACKAGE_DIR`):

1. IME client bound first.
2. QLineEdit `setFocus` → `text-input-v2 enable`, IME `Activate`.
3. Same Keyboard actions as ADR-328/330.
4. stdout `QT_LINEEDIT_TEXT=hi!`.

## Decision

1. **Packed musl Qt 6.6.3 inserts OSK text** when the field is a
   focused `QLineEdit`. The Falkon toolkit is not the paint gap.
2. **Falkon LocationBar stays untyped** (ADR-335). Same class as
   PCManFM PathEdit/Filter vs the Qt 5 probe.
3. **Do not flash panther.**

## Consequences

- APP-04 host Qt path is glibc Qt 5.15, packed musl Qt 5.15, and
  packed musl Qt 6.6.3 probes. Chrome widgets still do not paint.
- Rollback: drop `osk_ime_types_hi_bang_into_packed_qt6_lineedit`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
