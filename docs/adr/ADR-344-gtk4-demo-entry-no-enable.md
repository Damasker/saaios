# ADR-344: gtk4-demo `--run=entry` binds v3; center click does not enable

## Статус

Принято, 2026-09-21. APP-04 GTK 4.14 chrome on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-343 следующий свободный номер — **344**. Не S33.

## Контекст

ADR-343: a packed musl GTK 4.14.4 `GtkEntry` with `grab_focus` types
OSK `hi!`. That is a probe, not GTK demo chrome. Packed
`gtk4-demo --run=entry` is the same 4.14.4 binary as ADR-320.

Host qemu, windowed 1280×800, one `inject-click 640 400` (window
center, not a Y sweep):

1. Demo maps and commits a hashed shm frame.
2. After the click, compositor logs `text-input-v3 get`.
3. No `text-input-v3 enable` in 700 ms. IME stays inactive.
4. Extra cursor `wl_surface` hashes (`849af246…` / `8c6de10e…`) are
   not the Entry.

GTK chrome without `grab_focus` is the same class as Qt
PathEdit/LocationBar: protocol bind is not a focused insert target.

## Decision

1. **Do not claim gtk4-demo Entry was typed.** Bind ≠ enable.
2. **Do not sweep Y.** Center click is the one sample.
3. **Do not flash panther.** AUTH-10 stays a later flash-week device
   pass (reboot drops dest-no-lock).

## Consequences

- APP-04 packed GTK 4.14 probe stays typed. gtk4-demo chrome stays
  unfocused. Next compositor IME ordering will not enable this demo.
- Rollback: drop the gtk4-demo entry test.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
