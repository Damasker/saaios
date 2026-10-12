# ADR-535: main window SHM uses two slots

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
The ADR-532 one-slot class also covered the xdg-window SHM path.
Unlock is not proven on panther. Not Visual v1 sign-off. Not
VUI-09 Acceptance.

## Нумерация

После ADR-534 следующий свободный номер — **535**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-532/534 closed lock, OSK, and status-bar one-slot pools.
`WindowHandler::configure` still dropped `self.buffer` on every
configure, including same-size. The next `draw()` then
`create_buffer` while the compositor still held the first slot.
`draw()` already skips when `pool.canvas` is None so it does not
grow the pool without bound; configure-drop reopened that hole.
`panic_immediate_abort` turns `expect("create buffer")` into
SIGTRAP. dest-no-lock stays.

## Решение

1. Size the startup window pool with `lock_shm_pool_bytes`.
2. Reset `self.buffer` only when the window size changes.
3. A missing SHM slot or attach failure skips the frame.
4. dest-no-lock stays. Do not flash.

## Откат

Restore the one-frame startup pool and configure `buffer = None`.

## Ссылки

- ADR-512, ADR-532, ADR-533, ADR-534
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
