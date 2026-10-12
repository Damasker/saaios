# ADR-532: lock SIGTRAP is a one-slot SHM panic

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Named the ADR-503/512 lock SIGTRAP class. Unlock is not proven on
panther. Not Visual v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-531 следующий свободный номер — **532**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`9551b854` used `panic_immediate_abort` and died SIGTRAP after the
first lock frame. ADR-503 rebuilt without the immediate-abort
feature, skipped boot lock, and left the cause unnamed. ADR-510
recovers a leftover lock client only when the marker is present.
ADR-512 forbids lifting `/run/saaios/dev-no-lock` until the class
is named.

The lock SHM path created `SlotPool` at exactly `width * height * 4`
and cleared `lock_buffer` on every configure. The compositor's
second configure (or a redraw while the first buffer is still busy)
then `expect("create lock buffer")`. `panic_immediate_abort` turns
that into SIGTRAP. This is not an xkbcommon keymap trap (ADR-012)
and not a lock-formula change (ADR-232).

## Решение

1. Size the lock SHM pool for two page-aligned frames.
2. Reset `lock_pool` / `lock_buffer` only when the lock size changes.
3. A missing SHM slot skips the frame; it does not panic.
4. dest-no-lock stays. Do not flash. Do not convert leftover
   keyboard / gallery / lock formulas.

## Откат

Restore the one-frame pool and the configure `lock_buffer = None`.

## Ссылки

- ADR-012, ADR-013, ADR-232, ADR-503, ADR-510, ADR-512
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
