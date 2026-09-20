# VUI-09 verification ledger

Status: **in progress** (ADR-189). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile()` on `services/saai-shell/ui/root.sui`.
Do not point `build.rs` at `compile_v2()`. Space detail deferred.
MEM-08 omitted.

Current panther shell: `e8865301…` (ADR-188). Radio-off (ADR-191)
does not reflash.

## Matrix

| Area | Cell | Status | Evidence |
|---|---|---|---|
| Build | host tests | proven | overlay `cargo test -p saai-shell` 245; `saai-ui-compiler` 22 |
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
| Render | v2 layout ≡ v1 hits | open | `compile_v2()` has no rectangles (ADR-184) |
| Device | lock / unlock cycle | open | PIN null; this slice does not lock |
| Device | display restart | open | do not kill `saai-displayd` |
| Device | cold boot | open | not run on HEAD |
| Device | daylight / indoor / dark | open | no booth this session |
| A11y | 7-tap gallery | open | not this slice |
| Resilience | service restart | open | unlocked shell restart last done ADR-188 flash |
| Resilience | network / AI offline | proven | ADR-191; `wlan0` down → status `Нет сети`; Сейчас still live ObjectSummary; restored up |

## Still not Visual v1

- Declarative and procedural paths are not hit-test equivalent.
- Gallery covers fixtures; copying privileged names into an app fails
  `compile_v2_public()`, which is the gate, not a Stable API.
- ActionCard 38/27/25 and tab 31/27 stay named leftovers, not Title/Body.
- Space detail, Memory review, chat, widgets stay deferred.
