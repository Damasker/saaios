# ADR-324: IME commit_string reaches packed Qt v2; Qt does not repaint

## Статус

Принято, 2026-09-21. APP-04 Qt toolkit on host qemu. Not a panther
typed field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-323 следующий свободный номер — **324**. Не S33.

## Контекст

ADR-323: packed Qt 5.15 `enable`s `zwp_text_input_v2` when
`QT_IM_MODULE` is unset. A custom v2 client already received IME
`commit_string` (ADR-319). That is not PCManFM.

Host qemu, IME client bound first, then packed `pcmanfm-qt`:

1. `text-input-v2 get`, `enable`, IME `Activate`. No
   `hide_input_panel` before the type.
2. IME `commit_string("hi!")` + `commit(0)`.
3. Compositor logs `text-input-v2 commit_string` and sends the v2
   event to the same client that `enable`d.
4. No new hashed shm frame after that event. Earlier frames
   (`58f45fc…` then `484823fc…`) already happened around map/enable.

## Decision

1. **APP-04 protocol for Qt is compositor-true on host.** Separate OSK
   IME can Activate off PCManFM `enable` and forward `commit_string`.
2. **Do not claim the Filter shows `hi!`.** Qt 5.15 did not commit a
   new buffer. Paint stays unproven. Do not invent a surrounding-text
   parser to paper over that.
3. **Do not flash panther.** Next displayd experiment still carries
   ADR-311 bounds + ADR-319 v2. Packed `launch.c` already unsets
   `QT_IM_MODULE` (ADR-323).

## Consequences

- Host OSK → Qt v2 is the same IME path as GTK v3 (ADR-270/322), with
  an extra paint gap on this toolkit.
- Rollback: drop the PCManFM IME test; keep v2 enable (ADR-323).

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
