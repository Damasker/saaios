# ADR-185: VUI-09 — public `.sui` v2 subset is capability-gated

## Статус

Принято, 2026-09-20. Third-party `.sui` v2 may name only the public
subset: ADR-180 vocabulary minus privileged names. Shell still
parses privileged types through `compile_v2()`. `compile()` and
`build.rs` stay on `sui 1` `root.sui`. No paint change, no new
daemon. Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-184 следующий свободный номер — **185**. Не S33.

## Контекст

ADR-180 named privileged composites and surfaces so they would not
silently become an app API. ADR-181 still compiles them through
`compile_v2()` and only marks `is_privileged()`. VUI-09 needs a
third-party gate that does not import `saai-shell` and does not
point `root.sui` at `compile_v2()`.

## Decision

1. **Public subset.** A name is public when it is an ADR-180
   primitive, composite, or surface and is not privileged. Deferred
   names stay outside the vocabulary. Promotion to Stable is not
   this slice — the subset is Experimental and named.
2. **`compile_v2_public()`** is the app path. It uses the same
   grammar as `compile_v2()` and fails privileged surfaces
   (`lock`, `diagnostic`, `gallery`) and privileged components
   (`OrbHost`, `SystemStatus`, `DecisionOverlay`, `CapabilityRow`,
   `TrustedClientRow`). `compile_v2()` remains the shell path.
3. **No chrome switch.** `build.rs` still calls `compile()`. Gallery
   fixtures may include privileged rows; those rows are not public.

## Consequences

- An app document that names `OrbHost` fails at compile, not at
  paint. Rollback: keep `compile_v2()` unmarked for privileged
  names; drop `compile_v2_public()`.

## Verification

Host: `compile_v2_public` accepts the proven NOW v2 sample; rejects
`OrbHost` and `lock`; `compile_v2` of those still succeeds;
`compile()` stays v1. Panther: four v1 tabs on Сейчас, leave
Сейчас. No flash — the running binary is unchanged.
