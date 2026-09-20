# ADR-273: foreign OSK is a bottom Keyboard layer

## Статус

Принято, 2026-09-21. Host policy + shell IME client + layer chrome.
Not a panther flash. Not Visual v1 sign-off. Do not port wvkbd.

## Нумерация

После ADR-272 следующий свободный номер — **273**. Не S33.

## Контекст

ADR-270 mapped Keyboard keystrokes onto `commit_string`. ADR-271/272
sized and blitted a bottom layer. Shell Intent already paints an
in-window Keyboard. A third-party text-input field has no keys unless
the shell is also the IME client. This week's shell/displayd flash is
already spent.

## Decision

1. **Bottom layer.** Namespace `saai-shell-osk`, height of the Intent
   keyboard, `Anchor::BOTTOM`. Exclusive zone matches that height.
2. **IME activate only.** `zwp_input_method_v2` Activate maps the
   layer; Deactivate/Unavailable/session lock unmaps it. A shell Field
   (Intent, Wi-Fi password, PIN) keeps the in-window Keyboard.
3. **Same keys.** Taps go through `apply_osk_action` → `OskImeOp`.
   No `zwp_virtual_keyboard_v1`. Bind of the IME manager is optional
   so today's panther compositor (no global) still starts.
4. **Host-only this slice.** Do not flash `saai-shell` or
   `saai-displayd`.

## Consequences

- Visible OSK on panther still waits for a displayd+shell experiment.
- Rollback: drop `osk_layer.rs` and the IME bind.

## Verification

Host: `osk_docks_at_the_bottom_of_the_panel`,
`foreign_ime_activate_shows_the_layer`,
`shell_owned_field_keeps_the_layer_hidden`,
`hi_bang_from_layer_keys_reaches_the_ime_buffer`.
No panther flash. Leave Сейчас.
