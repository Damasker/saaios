# ADR-357: packed PCManFM v2 stays enabled without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-356 следующий свободный номер — **357**. Не S33.

## Контекст

ADR-353: packed PCManFM on `SAAIOS_SEAT_NO_KEYBOARD=1` enables v2
after xdg Activated, no click. ADR-337: packed Falkon URL v2
disables inside 80 ms quiet after enable on a keyboard seat.
Probe `QLineEdit` types (ADR-352). PathEdit/Filter still not typed
(ADR-341).

Same PCManFM package, `ShowFilter=true`, no click, no Ctrl+L, no
OSK: after the first `text-input-v2 enable`, 2 s quiet.

1. Framed, `xdg activated`, `focus set to`. No `keyboard focus set`.
2. `text-input-v2 enable`.
3. No `text-input-v2 disable` in those 2 s.

This chrome IM context is durable. Falkon URL disable (ADR-337)
does not transfer. Enable is still not a typed Filter/PathEdit.

## Decision

1. **PCManFM v2 on a panther-class seat stays enabled after
   Activated.** Do not treat it as Falkon 80 ms disable. Do not
   claim PathEdit/Filter typed.
2. **Do not click. Do not OSK. Do not add a fake `wl_keyboard`
   (ADR-012). Do not flash panther this week.** Next displayd
   flash must carry ADR-311+319+328+339+352. AUTH-10 stays a
   later flash-week device pass.

## Consequences

- APP-04 Qt chrome can keep v2 live without a keyboard. Insert
  still needs a focused `QObject`.
- Rollback: drop
  `packed_pcmanfm_v2_stays_enabled_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame
packed_pcmanfm_v2_stays_enabled_without_seat_keyboard`. dest-no-lock
kept. Do not flash. Leave Сейчас.
