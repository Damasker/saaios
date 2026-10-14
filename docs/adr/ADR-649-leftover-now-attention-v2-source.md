# ADR-649: leftover now_attention_v2_source is gone

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-648 следующий свободный номер — **649**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Live NOW attention / weather already sit on
`now_body_v2_source`. The attention-only alias was leftover.
Leave Сейчас. Font compare stays a device task. ADR-213 thin
tuning is not this slice. Do not invent weather.

## Решение

1. Drop `now_attention_v2_source`.
2. Attention and weather paint/hits call
   `now_body_v2_source(..., 0, false, 0, false)`.

## Откат

Restore the leftover alias.

## Ссылки

- ADR-648
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
