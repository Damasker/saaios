# ADR-327: PathEdit click re-enables v2; Qt still does not paint IME text

## Статус

Принято, 2026-09-21. APP-04 Qt field on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-326 следующий свободный номер — **327**. Не S33.

## Контекст

ADR-324: IME `commit_string` reaches packed PCManFM's v2 field; no new
shm. ADR-325: a focused GTK 4.18 Entry does paint `hi!`. ADR-326:
Ctrl+I (Show/Focus Filter) then `text-input-v2 disable` with no second
`enable`. Filter `lostFocus` is a dead end. Ctrl+B with
`ShowFilter=true` already on toggles the bar off. Racing IME between
Ctrl+I `update_state` and `disable` never logs a second
`commit_string` (PCManFM already queued disable).

Host qemu, IME client bound first, packed `pcmanfm-qt` with
`PathBarButtons=false` and `ShowFilter=true`:

1. First `enable`, IME `commit_string("hi!")` (ADR-324, still no new
   shm).
2. `inject-click 400 40` onto the focused toplevel (toolbar / PathEdit
   band). A second `wl_surface` maps with cursor hashes `64156441…`
   then `92cea996…` — that is the pointer cursor, not a Filter popup.
3. Qt sends `disable`, then `enable` (focus moved).
4. After 400ms quiet, IME `commit_string("hi!")` again. Compositor
   forwards it. Qt then `update_state` ×3.
5. Main shm stays `484823fc…`. No hashed PathEdit/Filter paint.

## Decision

1. **Toolbar click is a real v2 re-enable on host.** APP-04 may use
   `inject-click` (host stdin, not panther) to move Qt focus onto a
   text field after the first enable.
2. **Do not claim PathEdit or Filter shows `hi!`.** `update_state`
   after `commit_string` is not a new buffer. Qt 5.15
   `qwaylandtextinputv2` still drops or applies without damaging shm
   (`m_resetCallback` or `focusObject()`). Do not invent a surrounding-text
   parser to paper over that.
3. **Do not flash panther.** Next displayd experiment is still
   ADR-311 bounds + ADR-319 v2.

## Consequences

- Host OSK → Qt v2 can Activate off a click, not only off first map.
- Paint into packed Qt remains unproven. Next host probe is a QLineEdit
  that prints `textChanged`, not more PCManFM shortcut races.
- Rollback: drop the click half of `pcmanfm_frame`; keep ADR-323–326.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
