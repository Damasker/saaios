# ADR-183: VUI-09 — keep `.sui` v1 as the rollback artifact

## Статус

Принято, 2026-09-20. Production chrome compiles through `compile()`
on `services/saai-shell/ui/root.sui`. That file is the v1 rollback
artifact. `compile_v2()` must not replace it. No migration rewrite,
no paint change, no new daemon. Do not tap Inbox rows. Leave Сейчас.

## Нумерация

После ADR-182 следующий свободный номер — **183**. Не S33.

## Контекст

VUI-09 now parses `sui 2` names and properties, but the live shell
still draws from the v1 root schema (content actions + four tabs).
Switching `build.rs` to `compile_v2()` would drop that chrome
without a layout/hit-test path. The preserve has to be a named
artifact, not an unmarked habit.

## Decision

1. **`root.sui` is the rollback artifact.** `compile_v1_rollback()`
   compiles that exact source through `compile()`. Four tab labels
   and the two leftover NOW actions are the golden.
2. **`build.rs` calls `compile()`, not `compile_v2()`.** A host
   test reads the build script. Comments may name ADR-183; generated
   `root_sui.rs` does not change.
3. **No v1→v2 migrator in this slice.** v2 is not a production
   document yet. Layout/hit-test still come from the shell's
   existing `Node` path, not from `SuiV2Screen`.

## Consequences

- A later layout slice can replace `compile()` only after v2 emits
  the same rectangles. Rollback: keep this call site.

## Verification

Host: `compile_v1_rollback()` matches the four Russian tabs;
`compile_v2` of the same source fails; `build.rs` still contains
`saai_ui_compiler::compile(`. Panther: four v1 tabs on Сейчас,
leave Сейчас. No flash — the running binary is unchanged.
