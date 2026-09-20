# ADR-270: OSK is the existing Keyboard, text to foreign fields via IME

## Статус

Принято, 2026-09-21. Host protocol + keystroke mapping. Not a panther
flash. Not Visual v1 sign-off. Do not port wvkbd/Squeekboard.

## Нумерация

После ADR-269 следующий свободный номер — **270**. Не S33.

## Контекст

APP-04 asked for a visible on-screen keyboard that types into a
third-party field. ADR-029 already proved the shell Keyboard by
hit-test. ADR-022 / ADR-012 killed `libxkbcommon` and therefore
`zwp_virtual_keyboard_v1` plus stock wvkbd/Squeekboard. ADR-267 gave
`commit_string` on a seat with no keyboard. APP-03's host test used
one client for both the field and the IME. A real OSK is a second
client. This week's shell/displayd flash is already spent.

## Decision

1. **Same keys.** `Keystroke::to_ime_op()` maps the ADR-029/222 OSK
   actions onto `zwp_input_method_v2`: `Char`/`Enter` → `commit_string`,
   `Backspace` → `delete_surrounding_text(1, 0)` (current QWERTY is
   ASCII). Mode toggle and Escape stay local.
2. **Separate client.** The IME object lives on a different connection
   from the text-input field. `Keyboard::bind_foreign_ime()` is the
   loc for that sink; it is not a shell `Field`.
3. **Not wvkbd.** No `zwp_virtual_keyboard_v1`. No keymap.
4. **Host-only this slice.** Do not flash `saai-shell` or
   `saai-displayd`. Visible chrome on panther waits for the next
   shell experiment.

## Consequences

- A Qt/GTK field can receive OSK text without xkbcommon once the
  compositor binary from ADR-267 is on device.
- Shell paint of the panel over a foreign field is not in this slice.
- Rollback: drop `OskImeOp` and `tests/osk_ime.rs`.

## Verification

Host: `foreign_ime_osk_types_hi_bang_without_a_bound_field` in
`saai-ui-core`, and `separate_osk_client_types_hi_bang_into_foreign_field`
against a spawned `saai-displayd`. Sequence is `h`, `i`, backspace,
`i`, `!` → `hi!`. No panther flash. Leave Сейчас.
