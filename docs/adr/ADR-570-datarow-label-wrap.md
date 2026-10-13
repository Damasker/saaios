# ADR-570: DataRow long content reflows

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-569 следующий свободный номер — **570**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-569 did
this for Field. Section 6.8 DataRow still needed "Long content
reflows vertically." Value stays a trailing identifier. Metric /
Disclosure / StatusIndicator stay open. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. Primary and secondary wrap at two lines through `wrap_words`.
2. Visual height is 48 for one primary line and 64 when a second
   line exists. Existing `min_hit_height` still keys off secondary.
3. Value does not wrap. Accessibility keeps the full primary.
4. Gallery paint is not wired. Do not shrink `TextRole`.

## Откат

Remove DataRow wrap helpers.

## Ссылки

- ADR-569, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
