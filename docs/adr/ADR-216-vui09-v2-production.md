# ADR-216: VUI-09 — production `root.sui` is `compile_v2()`

## Статус

Принято, 2026-09-20. Trailing list rows (ADR-214) and Me
flatten/scroll at offset 0 (ADR-215) are host-equivalent.
Production `build.rs` compiles `services/saai-shell/ui/root.sui`
through `compile_v2()`. `root_view` uses `layout_v2()`. v1 chrome
stays frozen as `compile_v1_rollback()`. Nested `BottomNavigation`
tabs match live v1 hits. Content paint and Me/list dispatch stay
procedural. No new daemon. Leave Сейчас. Not Visual v1 sign-off.
Nothing Stable.

## Нумерация

После ADR-215 следующий свободный номер — **216**. Не S33.

## Контекст

`layout_v2()` now matches tab, footer, object, stacked, trailing,
and rest-state Me hits. `root.sui` was still `sui 1` empty content
plus tabs. Switching only the production document and `root_view`
is the chrome path change. Thin tuning stays last (ADR-213).

## Decision

1. **Document.** `root.sui` is `sui 2` with public surface `root`
   and `BottomNavigation` tabs `now` / `inbox` / `spaces` / `me`.
   No invented content rows. `root` is the proven v1 screen id.
2. **Build.** `build.rs` calls `compile_v2()`. Tab labels/icons are
   the proven v1 strings. `ROOT_TAB_HEIGHT` stays 300.
3. **Runtime.** `root_view` is `layout_v2(compile_v2(root.sui))`.
4. **Rollback.** `V1_ROLLBACK_SOURCE` is the frozen v1 text, not
   the live file.
5. **Reflash panther.** Chrome path changed. PIN stays null.

## Consequences

- Production tabs are declarative. NOW/Me/list bodies stay
  procedural until those screens are live v2 documents. Rollback:
  restore v1 `root.sui` and `layout_v1_root`. Still not Visual v1
  sign-off. Experimental stays Experimental.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-shell`; production
root tab hits match frozen v1. Panther: flash; Сейчас; marker on;
PIN null.
