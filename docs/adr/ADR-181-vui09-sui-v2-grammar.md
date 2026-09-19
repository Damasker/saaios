# ADR-181: VUI-09 — parse `sui 2` component composition

## Статус

Принято, 2026-09-20. `compile_v2()` accepts `sui 2` screens whose
`component` names are in the ADR-180 vocabulary. `compile()` still
builds only `sui 1` `root.sui`. Empty property blocks this slice;
tokens, insets, scroll, loc, focus, and accessibility stay later.
No paint change, no new daemon. Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-180 следующий свободный номер — **181**. Не S33.

## Контекст

ADR-180 named the proven types and surfaces but left `sui 2`
unparsed so the shell could not accidentally switch chrome. VUI-09
needs a versioned grammar before tokens or layout move. The grammar
must reject deferred names (`SpaceDetail`, Memory, chat, widgets)
instead of growing a second widget set.

## Decision

1. **`compile()` stays v1.** `build.rs` still compiles `root.sui`
   through `compile()`. A `sui 2` file there remains an error.
2. **`compile_v2()`** reads `sui 2` / `screen <surface> { component
   TypeName {} … }`. The screen id must be an ADR-180 surface. The
   type must be an ADR-180 primitive or composite. Empty `{}` is
   required so later slices can add properties without a silent skip.
3. **Unknown and deferred names fail at compile.** Privileged names
   (`OrbHost`, lock, …) parse and are marked privileged; they are
   not a public subset.

## Consequences

- Shell chrome does not read v2 yet. Rollback: drop `compile_v2()`.

## Verification

Host: a `now` screen of `ContextHeader` / `ObjectSummary` /
`BottomNavigation` compiles; `SpaceDetail` and unknown types fail;
`compile()` still rejects `sui 2`. Panther: four v1 tabs on Сейчас,
leave Сейчас. No flash — the running binary is unchanged.
