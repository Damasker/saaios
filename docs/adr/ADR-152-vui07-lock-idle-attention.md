# ADR-152: VUI-07 lock idle -- essential attention, presence only

## Status

Accepted, 2026-09-22. Host-verified only (2 new/updated tests, full
workspace test + clippy clean). Not yet physically re-confirmed on
Pixel 7 -- see Verification.

## Context

VUI-07's "Restyle remaining lock and wake-on-touch states" task, and
Visual Language 9.6's own spec for the lock screen: "prioritizes time,
device state, essential attention, and a clear unlock affordance."
ADR-134 (the no-PIN lock's own restyle) implemented time and the
unlock affordance, but explicitly left "essential attention" out --
its own Decision section says the hint is "the real no-PIN affordance,
not invented attention," treating the absence of any attention signal
as the safe default rather than fabricating one.

By now a real, honest, content-free attention signal already exists
and is already used elsewhere in this exact shell:
`saai_attention::has_orb_attention(&AttentionProjection) -> bool`,
which the bottom-navigation orb already lights up with post-unlock.
It is a pure boolean over waiting-confirmation tasks and undismissed
notifications -- never a title, count, or which Space something came
from. Showing *that* attention exists, without showing *what* it is,
is exactly the "essential attention without exposing bodies" the
roadmap's own task wording asks for, and is the same trade-off most
phone lock screens already make with notification dots/badges even in
a "hide content" privacy mode -- it is not the same category of
exposure as showing a Space name or an Inbox row, which ADR-134/148's
own "no Space name/Inbox content before unlock" boundary was actually
about.

A broader idea was raised alongside this task: a general lock-screen
*widget* system, where each widget declares and enforces its own
content-visibility security policy. That is real, valuable, future
work -- and a materially larger one (widget types, a policy engine,
a registry) than this ADR. Deliberately not attempted here; recorded
as its own roadmap item instead of folded into this narrow fix.

## Decision

**`draw_lock_idle` gains one new parameter, `has_attention: bool`.**
When true, draws a single small square dot (this codebase's own
"dot" convention -- `fill_rect`, no circle primitive exists) in
`ColorRole::Attention`, proportionally placed below the clock/hint.
When false, draws nothing -- not a dimmed or empty-state placeholder.
`ColorRole::Attention` was chosen over `Accent` specifically so this
reads as the same *kind* of signal the orb already uses for attention,
not a generic highlight.

**Computed in `present_lock_pin_entry`** (the one function that draws
either lock variant): `has_orb_attention(&project_from_entities(&self.
selected_entities))`, unconditionally -- cheap, no I/O, `selected_
entities` is kept live by `poll_entityd` regardless of lock state (the
main event loop polls it every tick, not gated on `self.locked`), so
the signal is never stale by more than one poll interval even while
locked.

**Drawn before the `fonts` check inside `draw_lock_idle`**, so the dot
still appears even with no font loaded, matching this function's own
existing "Canvas fill happens regardless of fonts" behavior.

**Not done**: the widget/policy system described above. **Not done**:
extending this to `draw_lock_pin_entry` -- that screen already shows
keypad chrome and dot-progress; adding a second, differently-shaped
dot there risks visual confusion with the PIN progress dots
themselves, and the no-PIN lock is the screen ADR-134's own "essential
attention" language was written against in the first place. Worth its
own look if this pattern proves right, not assumed here.

## Verification

- `cargo test -p saai-shell`: 198/198 (197 + 1 new --
  `lock_idle_attention_dot_only_draws_when_true`, checking the same
  pixel position reads `ColorRole::Attention` when true and plain
  `ColorRole::Canvas` when false). The existing
  `lock_idle_fill_is_canvas_not_the_diagnostic_red` test updated for
  the new parameter, no assertions weakened.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: unchanged pass count elsewhere (77
  test-result blocks, all `ok`).
- Not yet physically re-confirmed: needs an on-device check with a
  real waiting-confirmation task or undismissed notification present,
  screen locked (no PIN), confirming the dot appears, and confirming
  it disappears once that item clears.

## Consequences

- The no-PIN lock screen now honestly reflects Visual Language 9.6's
  "essential attention" requirement, closing the one part of that
  spec ADR-134 had left open on purpose.
- The dot is presence-only by construction -- there is no code path
  that could grow it into showing a count or title without a deliberate
  further change, unlike a field that starts empty and quietly gets
  filled in later.
- A general lock-screen widget/policy system remains real, valuable,
  unscoped future work -- recorded in the roadmap, not silently
  dropped, same as every other "found, not done here" item this
  sprint's own audits have already surfaced (Wi-Fi/Bluetooth offline
  gaps, ADR-150's Consequences).

## Rollback

`saai-shell.pre-lockattn` remains on-device once deployed. Purely
additive drawing logic gated on a new boolean parameter -- no data,
settings, or protocol changes.

## Links

- ADR-134 -- the no-PIN lock restyle this ADR completes the
  "essential attention" part of.
- ADR-114 -- the "cannot tell vs. confirmed nothing pending" honesty
  principle this ADR's "draw nothing, not a placeholder" choice
  follows.
- `crates/saai-attention` -- `has_orb_attention`/`project_from_entities`,
  the pre-existing, already-shipped signal this ADR reuses rather than
  computing its own.
