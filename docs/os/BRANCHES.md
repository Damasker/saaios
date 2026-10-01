# Branch triage — 2026-10-01

Baseline for comparison: `origin/feat/som-v1` (886 commits ahead of
`origin/main`, contains it entirely). `main` last moved 2026-09-03 and is
**behind the real product line**; fast-forwarding it to `feat/som-v1` is an
owner decision and is not done here.

"Lock" means GitHub *lock branch* (read-only, reversible, nothing deleted).
Apply with `.github/scripts/lock-stale-branches.sh --apply` (needs admin).

## Keep open

| Branch | Why |
|---|---|
| `feat/som-v1` | Active line, last commit 2026-10-01. |
| `main` | Default branch; stale, see above. |
| `feat/lockscreen-policy-widgets` | 2 unique lock-widget commits (ADR-425, `LOCK-WIDGETS.md`, `lock_widgets.rs`); merges into `som-v1` without conflicts. Also carries the 2 commits of PR #40. |
| `port/adr150-151-onto-som-v1` | Open PR #40 (ADR-424 trusted-client revoke). |
| `research/panther-modem-20260924` | 22 unique modem-bring-up commits. Diverged from `som-v1` (add/add conflicts in `MODEM-RUNTIME-2026-09-24.md`, `cp-boot-probe.c`, `sit-sim-status.c`), so it needs a human decision: port or retire. Not locked because it holds the only copy of that work. |
| `wip/local-pixel7-orphan` | Holds SM-A127F modem notes (`docs/os/targets/sm-a127f/modem.md`, +2586 lines) that exist nowhere else. Move the notes into `docs`, then lock. |
| `feat/pixel7-native-saaios` | Pre-split ancestor of the VUI-07 work. ADR-150/151 are re-landed by PR #40 as ADR-424; its ADR-149…152 numbers collide with different ADRs on `som-v1`. Lock after PR #40 merges. |

## Lock (stale)

| Branch | Evidence |
|---|---|
| `cursor/*` (7 branches) | PRs #11–#17 merged; each is an ancestor of `main`. |
| `phase-d-poweroff-gesture` | PR #21 merged; 0 unique patches. |
| `feat/s02-s03-host-slice`, `feat/s02-wayland-host`, `feat/s03-pixel7-displayd` | 2026-09-06…09 prototypes. S02 and S03 are `Done` in `docs/os/sprints/README.md`; `saai-displayd` was restructured since (only 3–16 % of their added lines survive in the same files). PR #38 closed. |
| `feat/vui-04-navigation` | 1 commit (ADR-117). `SystemStatus` and `reduced_motion` already exist on `som-v1` (VUI-08, ADR-167…179); the commit conflicts in 4 files. Only the ADR-117 text is unique; it stays readable on the locked branch. |
| `rescue/saaios-*` (8 branches) | Single "rescue uncommitted changes" snapshots of stale worktrees from 2026-09-21. 86–100 % of their added lines are already present in the same files on `som-v1` (a2 99 %, a3 90 %, a5 96 %, b1 100 %, b2 86 %, vui06 90 %, vui06-flash 94 %, vui07 98 %). |

## How this was measured

- `git rev-list --left-right --count feat/som-v1...<branch>` for ahead/behind.
- `git cherry feat/som-v1 <branch>` for patch-equivalent commits.
- `git merge-tree --write-tree` for merge conflicts with `som-v1`.
- Share of each branch's added non-trivial lines that already exist in the
  same file on `som-v1`.
- `gh pr list --state all` for PR state.
