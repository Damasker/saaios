# ADR-352: xdg Activated without `wl_keyboard` lets Qt QLineEdit type OSK

## Статус

Принято, 2026-09-21. APP-04 compositor + Qt toolkit on host. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-351 следующий свободный номер — **352**. Не S33.

## Контекст

ADR-346/350: GTK Entry types OSK on `SAAIOS_SEAT_NO_KEYBOARD=1`.
ADR-347–349: Qt 5.15 and Qt 6.6.3 with `setFocus` bound v2 and did
not enable. ADR-351: a pointer click did not substitute
`wl_keyboard`. Panther has no keyboard (ADR-012).

`activate_toplevel` called `text_ime::on_focus` and, on a host seat
with a keyboard, `keyboard.set_focus`. Without a keyboard it only
logged `focus set to`. It never set `xdg_toplevel` state
`Activated`. Qt `QWaylandInputContext::setFocusObject` enables v2
when `focusWindow()` is active; that waits for Activated, not for
`wl_keyboard.enter`.

Host change: on focus, `states.set(Activated)` + `send_configure`
for the current toplevel; unset on the previous. Log
`xdg activated`. Same path on panther-hardware (unflashed).

Host qemu, IME bound first, keyboard-less seat:

1. `xdg activated` then `focus set to`. No `keyboard focus set`.
2. `text-input-v2 enable` + IME `Activate`.
3. OSK prints `QT_LINEEDIT_TEXT=hi!` for host glibc Qt 5.15, packed
   musl Qt 5.15.10, and packed musl Qt 6.6.3.

ADR-347–351 stay as the pre-Activated facts. Do not add a fake
`wl_keyboard`. Falkon LocationBar / PCManFM PathEdit stay chrome
gaps (`focusObject() == null`).

## Decision

1. **xdg Activated is window activation, not a keyboard.** Qt probes
   type on a panther-class seat once that bit is set.
2. **Do not claim a panther field was typed.** Next displayd flash
   must carry ADR-311 bounds + ADR-319 v2 + ADR-328 v2
   delete_surrounding + ADR-339 v2 commit queue + **ADR-352
   Activated**.
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass (reboot drops dest-no-lock).

## Consequences

- APP-04 Qt and GTK probes both type without `wl_keyboard` on host.
  Qt chrome still needs a focused `QObject`.
- Rollback: drop the Activated set/unset in `activate_toplevel`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime --test gtk4_ime`.
dest-no-lock kept. Do not flash. Leave Сейчас.
