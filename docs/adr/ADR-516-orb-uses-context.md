# ADR-516: Orb Attention uses the same ContextFrame as NOW

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock widgets keep the Global `orb_attention` helper. Lock remains a
defect track (ADR-512).

## Нумерация

После ADR-515 следующий свободный номер — **516**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-506 drops Background from Orb. Live NOW already builds
`AttentionContext`. Live OrbHost still called `project_with_health`,
so a waiting-confirmation Task in another space lit Attention.

## Решение

1. `live_orb_attention` / `orb_visual_state_with` take the same
   context as NOW paint.
2. Tests without a context stay Global (ATTN-04 fixtures).
3. Health stays Global and still lights Orb.
4. Lock idle/wake formulas are not converted.

## Откат

Pass `None` for context; restore Global-only Orb.

## Ссылки

- ADR-506, ADR-509, ADR-513
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-04 / ATTN-05
