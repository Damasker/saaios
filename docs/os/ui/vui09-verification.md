# VUI-09 verification ledger

Status: **in progress** (ADR-189). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile()` on `services/saai-shell/ui/root.sui`.
Do not point `build.rs` at `compile_v2()`. Space detail deferred.
MEM-08 omitted.

Current panther shell: `3850427a…` (ADR-198). ActionCard/tabs were
`5eb6a27f…` (ADR-197). Named sizes were `e8865301…` (ADR-188).
`from_safe` (ADR-195), nested tab ids (ADR-196), nested footer
rows (ADR-199), and the object hit (ADR-200) stay host. Leftover
text budget (ADR-201) is host-locked; apps grid is the device cell.
Inbox `EventRow` hits (ADR-202) stay host. Spaces `SpaceRow` hits
(ADR-203) stay host. Me `SettingRow` hits (ADR-204) stay host.
Wi-Fi `WifiRow` hits (ADR-205) stay host.
Bluetooth `BluetoothRow` hits (ADR-206) stay host.
Privileged `TrustedClientRow` hits (ADR-207) stay host.
Privileged `CapabilityRow` hits (ADR-208) stay host.

## Matrix

| Area | Cell | Status | Evidence |
|---|---|---|---|
| Build | host tests | proven | overlay `cargo test -p saai-shell` 250; `saai-ui-compiler` 37 |
| Build | pixel7 cross-build | proven | ADR-198 `3850427a…` |
| Render | Сейчас composition | proven | ObjectSummary + footer; leftover NOW cards gone (ADR-187) |
| Render | four tabs | proven | hits 135/405/675/945 y=2250 (ADR-184); Inbox tap ADR-188 |
| Render | semantic color | proven | Theme + `panel_pixel`; no production RGB (ADR-187) |
| Render | named text sizes | proven | `role_px` leftovers (ADR-188); ActionCard/tabs (ADR-197); status/keys `Label`/`Caption` (ADR-198); leftover PX below Caption (ADR-201) |
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
| Render | v2 layout ≡ v1 hits | host | ADR-194, ADR-195, ADR-196, ADR-199, ADR-200, ADR-202, ADR-203, ADR-204, ADR-205, ADR-206, ADR-207, ADR-208; `layout_v2` nested tab ids; nested `row` footer matches `now_footer_action_rect`; `ObjectSummary` hits `open_object`; `EventRow`/`SpaceRow`/`SettingRow`/`WifiRow`/`BluetoothRow`/`TrustedClientRow`/`CapabilityRow` match `stacked_row_rect`; `select_space`; `cycle_timezone`; `connect_wifi`; `pair_bluetooth`; `revoke_trusted_client`; empty nav/rows/object/Status invent none; `compile_v2_public` rejects privileged; `compile()` stays v1 |
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
- ActionCard title/button use `Label`; status and tab labels use
  `Caption` (ADR-197). Status time/battery are `Label`; keys are
  `Caption` (ADR-198). They are not Title/Body. Badge, gallery
  kicker/swatch, and app-tile leftovers stay named below Caption
  (ADR-201). Do not 7-tap.
- Space detail, Memory review, chat, widgets stay deferred.
- Known limitations and the Visual v2 backlog: [`vui09-known-limitations.md`](vui09-known-limitations.md) (ADR-193).
