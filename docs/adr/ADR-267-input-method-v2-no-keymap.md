# ADR-267: `input-method-v2` without a keyboard or keymap

## Статус

Принято, 2026-09-21. Host compositor protocol. Not a panther flash.
Not APP-04 (on-screen keyboard chrome). Visual v1 stays unsigned.

## Нумерация

После ADR-266 следующий свободный номер — **267**. Не S33.

## Контекст

ADR-022 left `zwp_input_method_manager_v2` unimplemented because
smithay 0.7 `GetInputMethod` does `seat.get_keyboard().unwrap()`.
Any keymap compilation SIGTRAPs on panther. APP-03 asked for a path
that delivers text to a client field without `wl_keyboard`/xkbcommon.
Smithay's text-input Enable is also discarded unless that IME
instance exists, so pairing smithay text-input with a hand-rolled IME
cannot light `active_text_input`.

## Decision

1. **Own both globals.** `saai-displayd` implements
   `zwp_text_input_manager_v3` and `zwp_input_method_manager_v2` in
   `text_ime.rs`. `GetInputMethod` never calls `get_keyboard()`.
2. **`commit_string` is the text path.** An enabled field on the
   focused surface receives `commit_string` + `done`. Keyboard grab is
   accepted as a no-op object and does not send a keymap.
3. **Host-only this slice.** Do not flash panther `saai-displayd`.
   APP-04 (visible OSK) waits. Popup geometry is not laid out.

## Consequences

- A Qt/GTK client can enable a field and receive text from an IME
  client without xkbcommon.
- Enable is applied immediately (not double-buffered like the spec);
  enough for the APP-03 spike, not a full IME.
- Rollback: restore smithay `TextInputManagerState` and drop the
  input-method global.

## Verification

Host: `advertises_input_method_v2`,
`get_input_method_does_not_require_keyboard`,
`commit_string_reaches_enabled_text_input` against a spawned
`saai-displayd`. Existing `text_input_v3_binds_and_enables_without_crashing_compositor`
still passes. No panther flash. Leave Сейчас.
