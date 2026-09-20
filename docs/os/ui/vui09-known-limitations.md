# VUI-09 known limitations and Visual v2 backlog

Status: **recorded** (ADR-193). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile()` on `services/saai-shell/ui/root.sui`.
Do not point `build.rs` at `compile_v2()`. Space detail deferred.
MEM-08 omitted until a shell-legal memory read exists.

Current panther shell: `e8865301…` (ADR-188). This slice does not
reflash, lock, reboot, or kill `saai-displayd`.

The verification ledger stays in
[`vui09-verification.md`](vui09-verification.md).

## Session-blocked live cells

These stay **open**. They are not proven by this pass.

| Cell | Why this session does not run it |
|---|---|
| lock / unlock cycle | `/run/saaios/dev-no-lock` skips boot lock and idle lock. Removing it is an operator action. PIN is `null`. Killing a locked shell without `session_lock.unlock()` wedges displayd. |
| display restart | Do not kill `saai-displayd` except recovery. |
| cold boot | `/run` is lost across reboot; the no-lock marker would drop. |
| daylight / indoor / dark | No booth this session. |
| 7-tap gallery | Not a DevSurface-chrome slice. Do not 7-tap. |

Unlocked `saai-shell` restart (ADR-192) is not a lock cycle and not a
display restart.

## Compiler and layout

- `compile_v2()` parses names and properties. It has **no rectangles**.
  Live tabs still come from `layout_v1_root()` over `compile_v1_rollback()`.
- `compile_v2_public()` is the third-party gate. It is Experimental,
  not Stable.
- `Node` layout is physical pixels. `SafeInsets` are logical. Mixing
  those units is not reconciled here.
- `root.sui` stays `sui 1`. Switching `build.rs` to `compile_v2()` is
  forbidden until a v2 emitter matches the v1 tab hits (ADR-184).

## Leftover paint

- ActionCard leftover sizes 38/27/25 and tab 31/27 stay named, not
  Title/Body (ADR-188).
- Gallery fixtures may show privileged rows. Copying those names into
  an app fails `compile_v2_public()`.

## Deferred surfaces

`SpaceDetail`, `MemoryReview`, `ChatThread`, and `Widget` stay outside
the `.sui` v2 vocabulary. Widgets may show time and charge only when a
later sprint names a legal consumer; they are not NOW chrome.

## Visual v2 backlog

Ordered. None of this is in flight.

1. Emit layout and hit-test from `compile_v2()` that match
   `layout_v1_root()` for the supported public names. Until then,
   `compile()` stays on `root.sui`.
2. Reconcile `Node` physical pixels with logical `SafeInsets`.
3. Keep leftover ActionCard/tab sizes on an explicit named list, or
   map them onto `TextRole` in a paint-normalization slice.
4. Promote public names from Experimental to Stable only after the
   Pixel 7 promotion checklist in the component library.
5. Add `SpaceDetail` / `MemoryReview` / `ChatThread` / `Widget` to the
   vocabulary only when a shell-legal consumer exists. MEM-08 stays
   omitted until that memory read exists.
6. Run the session-blocked v1 cells (lock cycle, display restart, cold
   boot, daylight booth, 7-tap gallery) as operator-approved device
   work. They are Visual v1 gates, not v2 features.

Rollback: delete this page. The ledger and `compile()` path stay.
