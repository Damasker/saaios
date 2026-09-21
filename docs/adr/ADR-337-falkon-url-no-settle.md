# ADR-337: Falkon URL v2 disables before an `update_state` settle

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-336 следующий свободный номер — **337**. Не S33.

## Контекст

Qt 6.6 `qwaylandtextinputv2.cpp` discards `commit_string` while
`m_resetCallback` is pending (`wl_display_sync` after
`update_state` with flags ≠ change). Packed Qt 6.6.3 `QLineEdit`
inserts OSK `hi!` (ADR-336) because `setFocus` stays enabled.
Packed Falkon URL click enables v2; OSK in that window is forwarded
and does not paint (ADR-334/335).

Host qemu, same `640 20` click, wait for enable + `update_state` +
IME Activate, then 80 ms quiet before OSK:

1. `enable`, `update_state`, `enable`, `update_state` ×2
2. **`disable` inside those 80 ms**
3. No remaining enable window to wait out `m_resetCallback`

Immediate OSK (ADR-334) is the only compositor-true window. A settle
wait hits disable, same class as Ctrl+L / Ctrl+I (ADR-332/326).

## Decision

1. **Do not wait past the first enable** for Falkon URL OSK. There is
   no resetCallback settle gap.
2. **Do not claim LocationBar was typed.** ADR-335 still holds.
3. **Do not flash panther.** AUTH-10 stays a later flash-week device
   pass (reboot drops dest-no-lock).

## Consequences

- APP-04 Falkon chrome cannot use the probe's enable-and-hold path.
- Rollback: none. OSK test stays immediate-on-enable.

## Verification

Host experiment `falkon_url_osk_hi_bang_reaches_v2` with an 80 ms
quiet after `update_state` logged `disable` and never reached OSK.
Restored immediate OSK. dest-no-lock kept. Do not flash. Leave Сейчас.
