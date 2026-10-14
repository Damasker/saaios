# ADR-612: live Object View DecisionOverlay facts paint as Body

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-611 следующий свободный номер — **612**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-157 required overlay facts as Body, not the Caption dump
used for activity / OAM. Object View still painted every fact
row as Caption. Buttons already wrap (ADR-611). `heading()`
stays unpainted. Fact-key dump stays for non-overlay rows.
Font compare stays a device task. ADR-213 thin tuning is not
this slice. Do not tap Подтвердить or Отклонить.

## Решение

1. `object_view_fact_paint` marks a row Body / TextPrimary when
   it is one of `DecisionOverlay::fact_lines`.
2. Other fact rows stay Caption / TextSecondary. Accessibility
   of the overlay is unchanged. Leave Сейчас.

## Откат

Restore the Caption `draw_text` of every Object View fact row.

## Ссылки

- ADR-157, ADR-611
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
