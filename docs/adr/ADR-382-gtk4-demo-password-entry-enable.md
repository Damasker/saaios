# ADR-382: packed gtk4-demo `--run=password_entry` enables v3 without a tap

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-381 следующий свободный номер — **382**. Не S33.

## Контекст

ADR-373: gtk4-demo `--list` has `search_entry` and
`password_entry`, not `entry`. ADR-374: `--run=search_entry`
enables v3 without a tap. Whether `password_entry` is the same
auto-enable class was unproven.

Same keyboard-less seat, IME bound first, `--run=password_entry`,
no click:

1. xdg Activated, focus without `wl_keyboard`.
2. `text-input-v3 enable`.

Same class as search_entry auto-enable, not `--run=entry`
bind-without-enable. Not typed. Packed `gtk414-entry` still
types. Do not more `--run=entry` center clicks.

## Decision

1. **Do not treat password_entry as bind-without-enable.** It
   enables v3 without a tap. Do not claim typed. Do not add a
   fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381. AUTH-10 stays
   a later flash-week device pass.

## Consequences

- APP-04 gtk4-demo password_entry remaining is OSK insert/paint,
  not enable. Rollback: drop
  `packed_gtk4_demo_password_entry_enables_v3_without_click`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_password_entry_enables_v3_without_click`.
dest-no-lock kept. Do not flash. Leave Сейчас.
