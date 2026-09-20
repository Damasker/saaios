# ADR-194: VUI-09 — `layout_v2()` matches v1 tab hits for public NOW

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`.
`layout_v2(compile_v2_public(now-public.sui))` hits the four tabs at
the same 1080×2400 points as `layout_v1_root()`. Content leaves are
not actionable. `build.rs` still calls `compile()`. Shell still
calls `layout_v1_root()`. No new daemon. Leave Сейчас.

## Нумерация

После ADR-193 следующий свободный номер — **194**. Не S33.

## Контекст

ADR-184 said a v2 emitter may replace v1 layout only after the same
tab hits (135/405/675/945, y=2250). ADR-193 named that as Visual v2
item 1. `SuiV2Screen` had names, not rectangles. Expanding the
grammar to nest tabs is later; this slice borrows tab ids/actions
from `compile_v1_rollback()` when `BottomNavigation` is present.

## Decision

1. **`layout_v2(&SuiV2Screen, width, height)`** lays out non-actionable
   leaves for every component except `BottomNavigation`, then the v1
   tab strip. Without `BottomNavigation`, there are no tab hits.
2. **Do not switch `build.rs` or `root_view`.** Matching tab hits is
   necessary, not sufficient, for production v2 chrome. Footer and
   object hits stay procedural.
3. **Do not reflash, lock, reboot, or kill displayd.**

## Consequences

- Public NOW has host hit-test equivalence for tabs. Rollback: drop
  `layout_v2()`. Next: logical insets, or operator lock/display/cold-boot.
  Still not Visual v1 sign-off.

## Verification

Host: public NOW `layout_v2` matches v1 tab hits; a header-only v2
screen invents none of them; `compile()` stays on `root.sui`.
Panther: Сейчас on HEAD; leave Сейчас.
