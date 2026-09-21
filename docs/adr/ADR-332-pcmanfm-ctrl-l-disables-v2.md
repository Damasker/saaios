# ADR-332: Ctrl+L into packed PCManFM disables text-input-v2

## Статус

Принято, 2026-09-21. APP-04 Qt PathEdit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-331 следующий свободный номер — **332**. Не S33.

## Контекст

ADR-331: Filter-band click re-enables v2 without a new shm. ADR-326:
Ctrl+I (Show/Focus Filter) disables v2 with no second enable. Packed
`QLineEdit` with `setFocus` types OSK `hi!` (ADR-330); packed PCManFM
after pointer click does not (ADR-327/329/331).

Ctrl+L is the documented PathEdit path: `QShortcut` →
`MainWindow::focusPathEntry` → `PathEdit::setFocus()` + `selectAll()`.
Host qemu `inject-ctrl-l` (`KEY_LEFTCTRL`+`KEY_L`):

1. First map `enable` is live; IME `commit_string("hi!")` reaches v2
   (ADR-324 still holds).
2. After inject: `update_state` ×2, **`text-input-v2 disable`**,
   `update_state` ×2.
3. No second `enable` in 700 ms. IME is inactive (`active` is None),
   so OSK after that cannot `commit_string`.

Same class as Ctrl+I: Qt disables IM while Ctrl is held and does not
re-enable after release unless focus changes again without modifiers.
`v2 enable` at map is not `QLineEdit::setFocus`.

## Decision

1. **Host displayd stdin may inject `ctrl-l`** (`not(panther-hardware)`
   only). Same debug seat as `inject-ctrl-i`.
2. **Do not claim PathEdit was typed.** Ctrl+L currently ends the v2
   session. Do not sweep more click Y coordinates (ADR-331 Decision 1
   still holds).
3. **Do not flash panther.** Next packed PCManFM typed-field path is
   not a modifier shortcut. Falkon URL `QLineEdit` on host qemu is the
   remaining APP-04 chrome option if PCManFM stays disable-only.

## Consequences

- APP-04 host Qt insert is still proven only for a probe `QLineEdit`,
  not for PCManFM PathEdit or Filter.
- Rollback: drop `inject-ctrl-l`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
