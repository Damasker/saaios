# ADR-100: Feather Icons as SaaiOS's line-icon family, wired through the existing text pipeline

## Status

Accepted, 2026-09-17. Physically verified on Pixel 7.

## Context

VUI-02's own audit already named this gap explicitly (`visual-language-v1.md`'s
table): "Icons | No unified icon asset pipeline | One line-icon family and
build synchronization checks." Only an `IconSize` sizing token existed in
`saai-ui-core`; no actual icon asset, no semantic icon API, no rendering path.

## Decision

**Font**: Feather Icons (Cole Bemis and contributors), via the
`niklauslee/feather-webfont` static TTF build (npm `feather-webfont@4.22.3`).
MIT licensed, ~280 icons, a single consistent stroke weight -- matches Visual
Language v1's "calm technical operating environment" far better than a filled
or multi-weight icon set, and Feather's own stated goal ("simply beautiful
icons") is exactly that register. 74 KB, smaller than either text face
already shipped (IBM Plex Mono is 174 KB). Stored as
`os/targets/panther/assets/fonts/FeatherIcons.ttf`, license as
`FeatherIcons-LICENSE.txt` (fetched from the icons' own source repo,
`feathericons/feather`, since the webfont build itself ships no LICENSE file
in its npm package -- the MIT terms are the underlying icon designs',
inherited by any font built from them). Provenance and hashes recorded in
`os/targets/panther/third_party/README.md`, the same convention ADR-096
established for Montserrat and IBM Plex Mono.

**Integration**: an icon is a glyph at a Private Use Area codepoint --
Feather's webfont build already assigns one per icon (`content: "\fXXX"` in
its generated CSS). This means the *entire* rendering path is already built:
`fontdue` loads `FeatherIcons.ttf` exactly like the sans/mono faces
(`Fonts::icons: Option<Font>`, same tolerant load-or-none pattern as mono),
and `draw_text`/`draw_text_centered` draw an icon exactly like a one-character
string. No new drawing code, no new hit-testing model, no new asset format.

**API**: `saai_ui_core::IconGlyph`, mirroring `TextRole`'s own boundary --
product code names a semantic icon (`IconGlyph::Wifi`), never a codepoint or
`FeatherIcons.ttf`. `IconGlyph::codepoint()` is the only place a raw `\u{...}`
value appears.

**Scope**: a small, curated set (`Backspace`, `Wifi`, `WifiOff`, `Bluetooth`,
`Battery`, `BatteryCharging`, `Lock`, `Check`, `X`, `ChevronRight`,
`ChevronLeft`, `ChevronDown`, `AlertTriangle`, `Settings`) matched to a real
near-term need, not the full ~280-icon set imported with no call site --
consistent with this codebase's stated preference for concrete increments
over speculative breadth.

**Fallback**: unlike mono text, an icon has no fallback that means anything --
a letter cannot substitute for a wifi or lock mark. A missing or invalid icon
font simply draws nothing at that one call site rather than failing the shell
or substituting misleading text.

## A real bug fixed as the pipeline's first proof

The PIN keypad's backspace key has always labeled itself with the Unicode
erase mark U+232B ("⌫") -- a real character, but one the Montserrat sans face
has no glyph for. `fontdue` was silently substituting its own missing-glyph
placeholder box, visible in ADR-099's very first device screenshot (the
lock-screen keypad) as an empty box where every other key showed a normal
digit. Not a new defect -- the icon pipeline's first real integration is what
made it visible and fixable rather than something the screenshot tool alone
would have caught without something to compare against.

Fixed by routing that one key through a new shared `draw_keypad_label` helper
(used by all three of this codebase's keypad-style screens that were found to
share the identical draw call: the lock screen's PIN entry, the PIN-setup
screen, and ADR-029's own bespoke text-entry keyboard) -- it checks for the
"⌫" label specifically and draws `IconGlyph::Backspace` (mapped to Feather's
`delete` icon, `` -- Feather has no icon named "backspace") through the
icon font instead of the sans face; every other key (an ordinary digit or
letter) is untouched.

## Verification

- `cargo test -p saai-ui-core`: 12/12 (new: every `IconGlyph` codepoint is
  distinct and inside the Private Use Area).
- `cargo test -p saai-shell`: 87/87, unchanged.
- `cargo clippy --all-targets -- -D warnings`: clean for both crates.
- Pixel 7 `pixel7` profile cross-build succeeded.
- Hot-swapped onto the live device (font asset pushed to `/saaios/fonts/`,
  binary `mv`'d over the running one, `kill`, native-init respawned the same
  pid slot -- confirmed by `sha256sum` and the new pid), previous binary kept
  as `saai-shell.pre-icons` for rollback.
- Real device screenshot (`screencap`, ADR-099) of the lock screen's PIN
  keypad: the backspace key's tofu box is gone, replaced by a correctly
  positioned, correctly colored (matches the digit keys' accent color) delete
  icon. Every digit key (1-9, 0) kept its exact position -- no regression from
  routing one key's label through a different font.

## Not verified by this ADR

- The bespoke text-entry keyboard (ADR-029) and the PIN-setup screen share
  the exact same fixed code path (`draw_keypad_label`) and therefore the same
  fix by construction, but neither was reached and screenshotted this pass
  (the text-entry keyboard needs an active intent-input flow; PIN-setup needs
  navigating to "change PIN" first). Same reasoning as ADR-099's own deferred
  items -- the code path is identical and tested, not a different, unverified
  implementation.
- Clean native boot image rebuild (this ADR hot-swapped the running binary
  and font asset directly, matching VUI-01/VUI-02/ADR-099's own precedent).
- Increased text-scale review; a cold reboot smoke test.

## Consequences

- VUI-02's icon-pipeline gap is closed at the foundation level: any future
  primitive (`StatusIndicator`, disclosure chevron, button leading icon) has
  a real, working asset and API to draw through, not just a size token.
- Extending `IconGlyph` with more of Feather's ~280 icons as real call sites
  need them is a one-line addition to `foundations.rs`'s `codepoint()` match
  and its exhaustiveness test -- no rendering or asset work required again.

## Rollback

`saai-shell.pre-icons` remains on-device for atomic rollback. Removing
`FeatherIcons.ttf` from `/saaios/fonts/` degrades every `IconGlyph` call site
to drawing nothing (the documented fail-soft behavior), not a crash. No
entity, protocol, storage, or authorization change is included.

## Links

- `docs/os/sprints/VISUAL-ROADMAP.md` -- VUI-02.
- `docs/os/architecture/visual-language-v1.md` -- the icon-pipeline gap this
  closes.
- ADR-096 -- the typography asset convention this follows.
- ADR-099 -- the screenshot tool that found the bug this fixes.
