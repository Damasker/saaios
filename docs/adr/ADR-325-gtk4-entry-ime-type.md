# ADR-325: OSK IME types into a focused GTK 4.18 Entry

## Статус

Принято, 2026-09-21. APP-04 GTK toolkit on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash displayd
this week.

## Нумерация

После ADR-324 следующий свободный номер — **325**. Не S33.

## Контекст

ADR-322: host GTK 4.18 `Entry` with `grab_focus` sends
`zwp_text_input_v3::enable`. ADR-270: a custom v3 field received OSK
`hi!`. That is not GTK.

ADR-324: packed PCManFM-Qt `enable`s v2, compositor forwards
`commit_string`, Qt does not insert. Qt 5.15 drops the event when
`QGuiApplication::focusObject()` is null or `m_resetCallback` is
pending. The Filter bar was shown, not focused. A tap is still
required; this slice does not invent one.

Host GTK 4.18 probe (`tests/gtk4_hello.py`, `GTK4_PROBE_HOLD=1`):

1. IME client bound first.
2. Focused `Gtk.Entry` → `text-input-v3 enable`, IME `Activate`.
3. Same Keyboard actions as ADR-270 (`h i !` through IME
   `commit_string` / `done`).
4. Entry `notify::text` prints `GTK_ENTRY_TEXT=hi!`.

## Decision

1. **APP-04 host GTK path types.** OSK IME inserts `hi!` into a real
   GTK 4.18 Entry on `saai-displayd`.
2. **PCManFM Filter stays untyped** until a focused QLineEdit (click
   or equivalent) plus the next displayd flash (ADR-311+319).
3. **Do not flash panther.**

## Consequences

- Host GTK typing is compositor-true. Panther 4.14 Entry on a
  touch-only seat is still unproven.
- Rollback: drop `gtk4_ime` / `GTK4_PROBE_HOLD`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
