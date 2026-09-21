# ADR-328: OSK IME types into a focused Qt 5.15 QLineEdit

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash displayd
this week.

## Нумерация

После ADR-327 следующий свободный номер — **328**. Не S33.

## Контекст

ADR-325: a focused GTK 4.18 Entry receives OSK `hi!` through IME
`commit_string` / `done` (v3). ADR-324/327: packed PCManFM enables v2
and forwards `commit_string`, but Filter/PathEdit do not paint.

Host Qt 5.15.15 `QLineEdit` (`tests/qt5_lineedit.cpp`, `setFocus`,
`QT_QPA_PLATFORM=wayland`, `QT_IM_MODULE` unset):

1. IME client bound first.
2. QLineEdit → `text-input-v2 enable`, IME `Activate`, hashed shm.
3. Same Keyboard actions as ADR-270/325 (`h i !` through IME).
4. First run without v2 `delete_surrounding` printed
   `QT_LINEEDIT_TEXT=hii!` — `commit_string` inserts; OSK backspace
   did not. KDE `zwp_text_input_v2.delete_surrounding_text` is
   `before_length`/`after_length` in bytes, applied on the following
   `commit_string`.
5. After forwarding IME `DeleteSurroundingText` onto that v2 event:
   `QT_LINEEDIT_TEXT=hi!`.

`wayland-generic` is not a QPA plugin name; the host plugin is
`wayland` (with `wayland-egl` also present).

## Decision

1. **APP-04 host Qt path types.** OSK IME inserts `hi!` into a focused
   Qt 5.15 QLineEdit on `saai-displayd`.
2. **IME backspace is forwarded to v2** with the KDE before/after
   byte counts. Do not invent a second backspace path.
3. **Packed PCManFM Filter stays untyped** (ADR-327). Panther field
   stays untyped until the next displayd flash (ADR-311+319+this).
4. **Do not flash panther.**

## Consequences

- Host GTK v3 and Qt v2 both type through the same OSK IME.
- Rollback: drop `qt5_ime` / `qt5_lineedit.cpp` and the v2
  `delete_surrounding` loop.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime --test gtk4_ime`.
dest-no-lock kept. Do not flash. Leave Сейчас.
