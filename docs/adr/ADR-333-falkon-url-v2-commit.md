# ADR-333: packed Falkon URL click enables v2; IME commit_string reaches it

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-332 следующий свободный номер — **333**. Не S33.

## Контекст

ADR-317: panther Falkon is fullscreen, so URL `QLineEdit` is off
screen; the `file://` `<input>` never enables text-input. ADR-332:
PCManFM PathEdit Ctrl+L disables v2. ADR-324: IME `commit_string`
reaches a live v2 field. Packed Falkon is Qt 6.6 (v1/v2/v4; no v3);
intersection with displayd is v2 (ADR-319).

Host qemu, windowed 1280×800, `hideTabsWithOneTab=true` so chrome is
the navigation toolbar only, `inject-click 640 20`:

1. Qt binds `text-input-v2 get`.
2. Click maps the cursor surface, then **`text-input-v2 enable`**
   (twice) and `update_state`.
3. IME `commit_string("hi!")` in that enable window is forwarded
   (`text-input-v2 commit_string`).
4. Waiting out the click settle lets Qt **disable** first; IME
   `active` is None and a late commit is dropped. Same race as
   sending OSK after Ctrl+I (ADR-326).

A click at `640 72` (below that toolbar) is the webview: `update_state`
without enable.

## Decision

1. **Do not claim the LocationBar shows `hi!`.** This is protocol
   (enable + forwarded commit), not a new shm of the URL text.
2. **Do not sweep more Y coordinates.** `640 20` after hiding the
   single tab is the URL band. `640 72` is the page.
3. **Do not flash panther.** Panther Falkon stays fullscreen; URL
   chrome is still off screen (ADR-317).

## Consequences

- APP-04 host Qt insert into Falkon chrome is still unproven paint.
  Packed QLineEdit (ADR-330) remains the only typed Qt field.
- Rollback: drop the second `falkon_frame` test.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
