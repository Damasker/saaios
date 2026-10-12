# ADR-503: VUI-09 — host chrome on panther under dest-no-lock

## Статус

Принято, 2026-10-12. `/data/saaios/system/saai-shell` is
`f00e571d…` (pixel7 musl, panic messages on). dest-no-lock is
set. displayd pid stayed. Lock still SIGTRAP — not this slice.
Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. No flash.

## Нумерация

После ADR-502 следующий свободный номер — **503**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The Oct-7 shell `e3aaf0fc…` was still on the phone. The first
`9551b854…` build used `panic_immediate_abort` and died SIGTRAP
after the first lock frame, then displayd hit the 3/3 budget.
COM13 + NCM `172.31.7.1` is the live path.

## Decision

1. **Rebuild** without `panic_immediate_abort`. `panic = abort`
   stays. PUT lands under `/data/saaios/incoming/`, then replace
   the busy binary via rename.
2. **`/run/saaios/dev-no-lock`.** Skip boot lock. Kill shell only.
   displayd pid 389 stays. Frames commit on Сейчас.
3. **Lock stays open.** Do not remove dest-no-lock to “try lock”.
   Do not flash. Do not convert leftover keyboard / gallery /
   lock formulas.

## Consequences

- Host 494–502 chrome can be seen on panther without lock.
  Rollback: `saai-shell.prev-vui09host` (`e3aaf0fc…`). Still not
  Visual v1 sign-off.

## Verification

Panther: hash `f00e571d9df7b2ce…`; log `dev_no_lock enabled`;
shell 19267; displayd 389; entityd connected; frames after
placeholder. Host: no new tests. Leave Сейчас.
