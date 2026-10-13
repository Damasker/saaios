# ADR-541: Sistema playback hides helper binary names

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-540 следующий свободный номер — **541**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-257 is presence only: tinyplay plus the test tone, no auto-play.
Система «Воспроизведение» still printed `tinyplay · test-tone` /
`Нет tinyplay`. Those are helper paths, not board words. «Готово»
would invent a live player.

## Решение

1. Ready is «есть». Missing is «нет».
2. Do not print `tinyplay` or `test-tone`. Do not invent voice or
   Ready.
3. Detection still requires both files.

## Откат

Restore `tinyplay · test-tone` / `Нет tinyplay`.

## Ссылки

- ADR-257, ADR-529
- `docs/os/sprints/PIXEL-PATH.md` E / playback
