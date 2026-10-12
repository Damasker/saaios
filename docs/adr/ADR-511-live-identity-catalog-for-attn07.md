# ADR-511: live `system.identity` catalog for ATTN-07

## Статус

Принято, 2026-10-12. Host. No new agent runtime. Shell does not
execute the tool. Lock SIGTRAP stays open.

## Нумерация

После ADR-510 следующий свободный номер — **511**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-508 hid `display.inspect` because live `ToolRegistry::new()` had
no provider. Runtime already installs the working `system.identity`
executor. ATTN-07 needs that name present for resolve/preflight, not
a second planner or invented board labels.

## Решение

1. `system_identity_spec` lives in `tool-registry`. `system-tools`
   uses the same spec.
2. Shell registers `CataloguedTool` with that spec only. Execute
   stays denied here; tap still opens the object.
3. Empty registry still produces no suggestion. A task is still not
   a display object.

## Откат

Restore empty `ToolRegistry::new()` in the three live OAM helpers.

## Ссылки

- ADR-119, ADR-477, ADR-508
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-07
