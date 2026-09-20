# SaaiOS `.sui` v2 public API

Status: **Experimental** (ADR-186). Nothing here is Stable.

Product source: [Visual Language v1](../architecture/visual-language-v1.md)

Contract source: [Component Library v1](component-library-v1.md)

Compiler: `compile_v2_public()` in `saai-ui-compiler`. Production
chrome still compiles through `compile()` on
`services/saai-shell/ui/root.sui`.

## Stability labels

| Label | Meaning | Compile path |
|---|---|---|
| Experimental | Public name. API may still change. | `compile_v2_public()` |
| Privileged | Shell chrome. Not an app API. | `compile_v2()` only |
| Deferred | Not in the vocabulary. | fails |
| Stable | Promotion checklist passed on Pixel 7. | none yet |

`sui_v2_stability(name)` returns the first three. Unknown names return
`None`.

## Public example

[`examples/now-public.sui`](examples/now-public.sui) is the labelled NOW
sample: `ContextHeader`, `ObjectSummary`, `SurfacePattern`,
`BottomNavigation`. It matches the live Сейчас composition (header,
object, empty pattern, tabs) without leftover v1 NOW cards and without
`OrbHost`.

```
compile_v2_public(include_str!("…/now-public.sui"))
```

Spec sheets for those types remain in the component library. This page
does not invent a second palette or a second layout engine.

## Gallery

`saai-ui-core::public_gallery_type_names()` lists public fixture types.
`privileged_gallery_type_names()` is `DecisionOverlay` and
`TrustedClientRow`. The device gallery may show privileged rows for
review. Copying them into an app document fails `compile_v2_public()`.

## Deprecation

- `root.sui` has no leftover NOW actions (ADR-187). Live NOW uses
  `ObjectSummary` and the footer. `inspect_selected_entity` is not a
  public name.
- Privileged names that appear in gallery fixtures are not promoted by
  that appearance.
- Removing a public name requires an ADR and a compile error, not a
  silent skip.
- `compile_v2()` on `root.sui` stays forbidden until a v2 emitter
  matches the v1 tab hits (ADR-184).

## Migration

| From | To |
|---|---|
| Copying `saai-shell` paint | `saai-ui-core` types + this example |
| `sui 1` leftover NOW cards | removed in ADR-187; public NOW sample above |
| `OrbHost` / lock / gallery in an app | omit; those stay privileged |
| Switching `build.rs` to `compile_v2()` | do not; keep `compile()` |

Space detail, Memory review, chat, and widgets stay deferred.
Known limitations: [`vui09-known-limitations.md`](vui09-known-limitations.md).
