# ADR-099: real device screenshots for physical review, and a found/fixed glyph baseline bug

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7.

## Context

VUI-02 (typography) needs "physical Pixel 7 review" to close its task list, but
every prior VUI physical review in this project's history was a human looking
directly at the phone -- a headless development session (no eyes on the
device) has never had a way to actually see what saai-displayd composited,
only frame-hash/log evidence and touch-injection outcomes. This is a real
verification gap: log evidence can confirm a frame was committed, not that it
looks correct.

## Decision

Added `os/targets/panther/tools/screencap.c`, a small diagnostic-only tool
(same category as `vk-triangle`, `vk-frame` in the same directory -- a
throwaway spike binary, not shipped in the image) that reads the active CRTC's
scanout framebuffer directly via legacy DRM/KMS ioctls
(`GETRESOURCES`/`GETCRTC`/`GETFB`/`MAP_DUMB`) and writes it as a 32bpp
top-down BMP. It runs independently of `saai-displayd` -- no IPC, no shared
state, read-only ioctls that do not disturb the running compositor or claim
DRM master. `DRM_FORMAT_XRGB8888` (the format saai-displayd's own logs already
name, `DrmFourcc(XR24)`) is byte-for-byte the same little-endian layout a
32bpp BMP uses, so the pixel data is copied straight through with no
conversion. `GETFB`'s `handle` field (needed to actually map the buffer) is
only populated for a caller with `CAP_SYS_ADMIN` or DRM master -- satisfied
here because it runs as root over SSH, regardless of which process currently
holds master.

This gives any future session (human or AI) a fast, non-disruptive way to
actually see the current screen: `screencap /dev/dri/card0 > shot.bmp`, pulled
off-device and converted to PNG with ImageMagick.

## Finding

The very first real screenshot (of the `Я` settings page) showed a visible
defect: the hyphen in "Wi-Fi" and "PIN-код" rendered as a short mark floating
near the top of the letters, well above where a hyphen belongs -- not a
missing glyph or wrong font, a mispositioned one.

Root cause, in `services/saai-shell/src/render.rs`'s `draw_text` and
`draw_text_centered`: every glyph's rasterized bitmap was blitted starting at
the same `top` row, with no vertical (baseline) offset applied at all --
`fontdue::Metrics::ymin` (the bitmap's bottom edge, in pixels above the
baseline) was read for nothing; only `xmin` was used for horizontal placement.
For an ordinary letter, whose tight bitmap already spans close to the full
cap-height-to-baseline distance, this is invisible. For a short glyph -- a
hyphen, period, colon, apostrophe -- whose tight bitmap is only a few pixels
tall, anchoring its bitmap's top at the same row as a full letter's top
placed it far above its correct mid-height position. Any text mixing a full
letter with a short punctuation mark was affected; this was not specific to
"Wi-Fi".

## Fix

Both functions now compute, once per call, the tallest glyph actually present
in that specific text run (`ymin + height`, in fontdue's own baseline-relative
units) and use it as a shared reference: each glyph's vertical position is
`top + (reference_height - (glyph.ymin + glyph.height))`. For the reference
glyph itself this is zero, so an ordinary line of same-height letters renders
at exactly its previous pixel position -- confirmed both by the math (this
project's dozens of existing `draw_text`/`draw_text_centered` call sites were
tuned by eye against the old behavior for ordinary letters, and this fix does
not move them) and by the after screenshot below, where "Wi-Fi", "05:04",
"99%", and every other line in the same view sit exactly where they did
before. Only glyphs shorter than their line's tallest glyph move, by exactly
the amount needed to land on the same baseline.

## Verification

- `cargo test -p saai-shell`: 87/87, unchanged from before the fix.
- `cargo clippy -p saai-shell --all-targets -- -D warnings`: clean.
- Pixel 7 `pixel7` profile cross-build succeeded
  (`build-saai-shell.sh`).
- Hot-swapped onto the live device (`mv` over the running binary, `kill`,
  native-init respawned the same pid slot -- confirmed by `sha256sum` and the
  new pid), with the previous binary preserved as
  `saai-shell.pre-baseline-fix` for rollback, the same convention VUI-01/
  VUI-02 already established (`.pre-vui01`, `.pre-vui02`).
- Before/after screenshots via the new `screencap` tool: the "Wi-Fi" hyphen
  moved from floating near cap-height to sitting at normal mid-height; every
  other visible glyph in the same view (digits, Cyrillic letters, `·`, `+`,
  `:`, `.`) kept its position, no new clipping or overlap introduced.
- The shell resets to the lock screen on every restart by design
  (`Shell::new()`'s `locked: true`) -- the device owner unlocked it manually
  for the after-screenshot; no PIN was requested or handled by this session.

## Not verified by this ADR

- ADR-096's own remaining checklist: exercising a real SSH pairing fingerprint
  in the mono role (the only current call site using `TextRole::MonoBody`)
  needs a live pairing handshake to reach that screen, not attempted this
  pass; rebuilding and re-verifying the clean native boot image (this ADR
  only hot-swapped the running binary, matching VUI-01/VUI-02's own
  precedent); increased text-scale review.
- A cold reboot smoke test (VUI-01's own Definition of Done included one) --
  this change is narrower in scope (a pure rendering calculation, no token/
  protocol/storage change) and was not repeated here; left as an open item if
  the project wants the same bar applied to every VUI-02 commit.

## Consequences

- Any future session, including a fully headless one, can now get real visual
  evidence instead of relying on frame-hash/log inference alone --
  `screencap.c` should be reused for the rest of VUI-02's own physical-review
  gates (component gallery, remaining primitives) rather than re-derived.
- The same baseline bug this fix addresses would have affected every future
  primitive that mixes full letters with short punctuation or symbols
  (dividers, `StatusIndicator` marks, disclosure chevrons rendered as text) --
  fixed once at the shared `draw_text`/`draw_text_centered` layer rather than
  worked around per call site.

## Rollback

`saai-shell.pre-baseline-fix` remains on-device for atomic rollback. No
entity, protocol, storage, token, or authorization change is included --
purely a glyph-positioning calculation inside the renderer.

## Links

- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
- ADR-096 -- the typography/mono-font work this review pass started from.
- `os/targets/panther/tools/screencap.c` -- the new capability.
