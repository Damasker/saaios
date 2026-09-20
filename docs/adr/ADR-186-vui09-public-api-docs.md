# ADR-186: VUI-09 — public API docs, gallery examples, stability labels

## Статус

Принято, 2026-09-20. The public `.sui` v2 subset has a host-tested
example, stability labels, and a migration/deprecation page.
`compile()` and `build.rs` stay on `sui 1` `root.sui`. No paint
change, no new daemon. Do not tap Inbox rows. Do not 7-tap the
gallery. Leave Сейчас.

## Нумерация

После ADR-185 следующий свободный номер — **186**. Не S33.

## Контекст

ADR-185 named the public subset and gated privileged names. VUI-09
still needed published API docs, a compilable example, gallery
stability labels, and a migration rule so third-party code does not
copy `saai-shell` or leftover v1 NOW cards.

## Decision

1. **Example.** `docs/os/ui/examples/now-public.sui` is the public
   NOW sample. `compile_v2_public()` accepts it. `compile()` rejects
   it as version 2.
2. **Stability.** `sui_v2_stability()` returns Experimental for
   public names, Privileged for the ADR-180 privileged list, Deferred
   for omitted names. Nothing is Stable in this slice.
3. **Gallery labels.** Public fixture types are listed separately
   from privileged `DecisionOverlay` / `TrustedClientRow`. Spec
   sheets stay in `component-library-v1.md`. Deprecation: leftover
   v1 NOW cards and privileged gallery rows are not an app API.
   Production chrome stays `compile()` on `root.sui`.

## Consequences

- Apps copy the public example, not `root.sui`. Rollback: drop the
  example and `sui_v2_stability()`; keep `compile_v2_public()`.

## Verification

Host: the example compiles through `compile_v2_public()`; every
component in it is Experimental; privileged gallery names are not
public; `build.rs` still contains `saai_ui_compiler::compile(`.
Panther: four v1 tabs on Сейчас, leave Сейчас. No flash — the
running binary is unchanged.
