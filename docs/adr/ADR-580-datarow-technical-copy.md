# ADR-580: DataRow technical values are mono and copyable

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance. Not VUI-05 Acceptance.

## Нумерация

После ADR-579 следующий свободный номер — **580**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-05 wrap leftovers are in (ADR-568–579). Section 6.8 still
required technical identifiers to use mono body and expose a copy
action when useful. Pair fingerprint paint already wraps MonoBody
(ADR-432). DataRow had no host contract for that. Object-view
fact-key dump stays. Gallery paint is not this slice. Font
compare stays a device task. ADR-213 thin tuning is not this
slice.

## Решение

1. `with_technical_value` sets the trailing value and
   `TextRole::MonoBody`. Ordinary `with_value` stays Body.
2. `with_copy` is inspect, not navigation. Static rows stay
   non-actionable. `is_copyable` requires both a value and a copy
   action.
3. Accessibility keeps the identifier. Gallery paint is not
   wired. Do not convert leftover formulas.

## Откат

Remove `value_mono`, `copy_action`, and the helpers.

## Ссылки

- ADR-432, ADR-570
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
