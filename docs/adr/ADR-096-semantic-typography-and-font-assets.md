# ADR-096: Semantic typography and reproducible font assets

## Status

Accepted, 2026-09-17.

## Context

Visual Language v1 defines stable semantic text roles while the current shell
still selects Montserrat fields and physical sizes at individual draw calls.
SaaiOS also needs a real monospace face for identifiers, commands, addresses,
logs, and telemetry. Reusing proportional sans or a terminal bitmap would
weaken hierarchy and repeat the DOS-like appearance the visual track rejects.

The renderer uses `fontdue`, which cannot consume the variable font assets used
by many current distributions. The selected assets must therefore be static,
licensed for redistribution, include Latin and Cyrillic (including Ukrainian),
fit the init-boot budget, and render without a font service.

## Decision

- `saai-ui-core::TextRole` is the product API. It maps to semantic family,
  weight, logical size, and line height. Callers do not name font files.
- Montserrat Regular/SemiBold remains the sans family during VUI-02. It was the
  user's physically selected S13 result and will be replaced only if a device
  comparison demonstrates a readability improvement.
- IBM Plex Mono Regular is the initial mono family. It is a static TTF with the
  required Cyrillic coverage, an industrial but readable character, and SIL
  Open Font License 1.1 redistribution terms.
- IBM Plex Mono is pinned to upstream commit
  `78cd4223d8de9fcb78cba84eadecb269c56093c5`; the font and license hashes live
  in the Panther third-party inventory.
- The clean image builder packages the font and its license in the same change
  as the runtime loader. Direct device installation is verification, not the
  source of truth.
- Missing or invalid mono falls back to regular sans with a diagnostic. Missing
  required sans remains a startup error. A mono packaging regression therefore
  cannot remove all text from the shell.

## Consequences

The semantic API can change backend assets without changing components. The
shell can begin migrating technical text independently from ordinary prose.
The boot image grows by approximately 174 KiB before compression. Full image
size and cold-boot verification remain VUI-02 gates.

This ADR does not declare the typography component stable, select a third-party
application font configuration, or change every legacy raw size at once. The
component gallery provides the comparison and migration surface.

## Verification

- Validate static font metadata and `ru`/`uk` coverage on the checked-in file.
- Run shell and workspace tests plus strict clippy.
- Build the static Pixel 7 shell with `fontdue`.
- Exercise a long SSH fingerprint in the mono role without clipping.
- Rebuild the clean native image and verify both the font and license entries.
- Review normal and increased text scale on the physical Pixel 7.

## Rollback

Remove the mono asset and image entries and resolve `MonoBody` through the sans
fallback. No entity, protocol, storage, or authorization data changes.
