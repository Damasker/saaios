# ADR-330: OSK IME types into a packed musl Qt 5.15 QLineEdit

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-329 следующий свободный номер — **330**. Не S33.

## Контекст

ADR-328: host glibc Qt 5.15.15 `QLineEdit` with `setFocus` receives
OSK `hi!`. ADR-329: packed PCManFM (musl Qt 5.15.10) gets that same
OSK sequence after a PathEdit click and does not attach a new
toplevel shm.

Packed probe is the same `tests/qt5_lineedit.cpp`, linked with zig
`aarch64-linux-musl` against the PCManFM package Qt and Alpine
`qt5-qtbase-dev` 5.15.10 headers. Run under `qemu-aarch64-static -L`
that package (`PACKED_QT5_LINEEDIT`, `PCMANFM_PACKAGE_DIR`):

1. IME client bound first.
2. QLineEdit `setFocus` → `text-input-v2 enable`, IME `Activate`.
3. Same Keyboard actions as ADR-328.
4. stdout `QT_LINEEDIT_TEXT=hi!`.

## Decision

1. **Packed musl Qt 5.15.10 inserts OSK text** when the field is a
   focused `QLineEdit`. The toolkit on the panther package is not the
   paint gap.
2. **PCManFM PathEdit/Filter stays untyped** (ADR-329). The remaining
   host gap is that client, not v2 forwarding and not musl Qt.
3. **Do not flash panther.** Panther still runs displayd without this
   host v2 `delete_surrounding` until the next compositor flash.

## Consequences

- APP-04 host Qt path is glibc QLineEdit and packed musl QLineEdit.
- Rollback: drop `osk_ime_types_hi_bang_into_packed_qt5_lineedit`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
