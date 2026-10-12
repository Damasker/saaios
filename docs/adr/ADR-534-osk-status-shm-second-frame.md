# ADR-534: OSK and status-bar SHM use two slots

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
The ADR-532 one-slot class also covered OSK and the status-bar
SHM fallback. Unlock is not proven on panther. Not Visual v1
sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-533 следующий свободный номер — **534**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-532 sized lock SHM for two page-aligned frames and kept the
buffer on a same-size configure. `present_foreign_osk` and the
status-bar `wl_shm` fallback still created a one-frame pool and
cleared `osk_buffer` / `layer_buffer` on every configure. The
same `expect("create … buffer")` is SIGTRAP under
`panic_immediate_abort`. This is not an OSK-formula change
(ADR-232) and not an unlock proof (ADR-512).

## Решение

1. Reuse `lock_shm_pool_bytes` / `lock_size_replaced` for OSK and
   the status-bar SHM path.
2. Reset those pools only when the layer size changes.
3. A missing SHM slot skips the frame; it does not panic.
4. dest-no-lock stays. Do not flash. Do not type Intent.

## Откат

Restore the one-frame OSK/layer pools and the configure
`*_buffer = None`.

## Ссылки

- ADR-232, ADR-503, ADR-512, ADR-532, ADR-533
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
