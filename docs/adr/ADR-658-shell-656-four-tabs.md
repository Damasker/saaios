# ADR-658: four root tabs after ADR-656 respawn

## Статус

Принято, 2026-10-14. Live `/data/saaios/system/saai-shell` stays
`eb3197c9…` (ADR-656). dest-no-lock stays absent. displayd pid
2694 stayed. New shell 12671. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked. VUI-04/05/06 Acceptance stay unchecked.

## Нумерация

После ADR-657 следующий свободный номер — **658**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-657 proved unlock + Now/Spaces/Search. A NOW Object View tap
at `(540, 335)` then ate later tab hits (modal owns touch,
including Система). That was not a leftover Me miss. The user
asked to continue frames. Kill the stuck shell only. Do not start
the shell from COM13. Do not tap Orb. Do not restore dest-no-lock.

## Решение

1. Kill shell 2396 only. displayd 2694 respawned 12671.
   dest-no-lock stayed absent. Fresh FramePace:
   `surface=lock` `backend=shm` seq=3.
2. Unlock `(540, 1200)`. Then tabs only, no Object View:
   Сейчас `(135, 2250)`, Пространства `(405, 2250)`,
   Поиск `(675, 2250)`, Система `(945, 2250)`.
3. Log: `switched to Search` then `switched to Me`. Status
   `1080x120` and main `1080x2400` blit on the Me tap.
4. MonoBody technical IDs still need a visible Sistema
   observation/`video*` row on glass. Copy stays off.

## Откат

No binary change. Previous binary remains
`/data/saaios/system/saai-shell.prev-adr614`.

## Ссылки

- ADR-656, ADR-657, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
