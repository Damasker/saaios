# VUI-09 verification ledger

Status: **in progress** (ADR-189). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile()` on `services/saai-shell/ui/root.sui`.
Do not point `build.rs` at `compile_v2()`. Space detail deferred.
MEM-08 omitted.

Current panther shell: `e8865301…` (ADR-188). Known limitations
(ADR-193) do not reflash.

## Matrix

| Area | Cell | Status | Evidence |
|---|---|---|---|
| Build | host tests | proven | overlay `cargo test -p saai-shell` 248; `saai-ui-compiler` 23 |
| Build | pixel7 cross-build | proven | ADR-188 `e8865301…` |
| Render | Сейчас composition | proven | ObjectSummary + footer; leftover NOW cards gone (ADR-187) |
| Render | four tabs | proven | hits 135/405/675/945 y=2250 (ADR-184); Inbox tap ADR-188 |
| Render | semantic color | proven | Theme + `panel_pixel`; no production RGB (ADR-187) |
| Render | named text sizes | proven | `role_px` + leftovers (ADR-188) |
| Input | tab switch | proven | Inbox then back to Сейчас on HEAD |
| Input | keyboard | proven | ADR-029/150/161; not re-typed this slice |
| Input | interrupted key | proven | ADR-163 |
| State | empty / offline patterns | host | SurfacePattern tests; live NOW empty is «Ничего срочного» |
| A11y | reduced motion | proven | ADR-174; not toggled this slice |
| A11y | focus order | proven | ADR-162 |
| A11y | increased text on panther | proven | ADR-190; HEAD `e8865301…` 150% then restored 100; no Me tap |
| Perf | frame pace / p95 / idle | proven | ADR-172–177; not re-read `/run/saaios/shell-frame.last` this slice |
| Perf | haptic policy | proven | ADR-171/179 KeyPress-only |
| Safety | public subset gate | host | `compile_v2_public()` (ADR-185/186) |
| Render | v2 layout ≡ v1 hits | open | `compile_v2()` has no rectangles (ADR-184); named in ADR-193 |
| Device | lock / unlock cycle | open | PIN null; marker skips lock; named in ADR-193 |
| Device | display restart | open | do not kill `saai-displayd`; named in ADR-193 |
| Device | cold boot | open | not run on HEAD; named in ADR-193 |
| Device | daylight / indoor / dark | open | no booth this session; named in ADR-193 |
| A11y | 7-tap gallery | open | not this slice; named in ADR-193 |
| Resilience | service restart | proven | ADR-192; unlocked kill, marker on, same `e8865301…`, Сейчас without lock |
| Resilience | network / AI offline | proven | ADR-191; `wlan0` down → status `Нет сети`; Сейчас still live ObjectSummary; restored up |

## Still not Visual v1

- Declarative and procedural paths are not hit-test equivalent.
- Gallery covers fixtures; copying privileged names into an app fails
  `compile_v2_public()`, which is the gate, not a Stable API.
- ActionCard 38/27/25 and tab 31/27 stay named leftovers, not Title/Body.
- Space detail, Memory review, chat, widgets stay deferred.
- Known limitations and the Visual v2 backlog: [`vui09-known-limitations.md`](vui09-known-limitations.md) (ADR-193).
