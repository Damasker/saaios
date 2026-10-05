# ADR-424: trusted-client revoke needs a confirming second tap, and never revokes the current session's own key

## Status

Accepted, 2026-09-22. Ported from `feat/pixel7-native-saaios`'s ADR-150
and ADR-151 (a separate, independently-continued lineage off the same
`a5a599b` ancestor this branch also shares), adapted to this branch's
current `sui-v2`/`layout_live_v2` rendering path. Host-verified only
(314/314 `saai-shell` tests, `cargo build --workspace` clean -- see
Verification for why `cargo clippy --workspace` itself could not be
used as the gate here). Not yet physically confirmed on Pixel 7.

## Context

`feat/pixel7-native-saaios` audited this exact screen (VUI-07's
"apply shared confirmation... patterns" task) and found
`handle_trusted_client_tap`'s `Revoke(index)` arm called
`revoke_trusted_client` unconditionally on a single tap anywhere on the
row -- `trusted_client_action_at` makes the whole card the tap target,
not a small button inside it. Revoking a trusted SSH client is
irreversible (rewrites `authorized_keys` on the spot) and specifically
dangerous for the person most likely to be using this exact screen: an
admin reaching the device over SSH right now, scrolling this list,
could revoke the very key carrying that session with one mis-aimed tap.

That branch's ADR-150 added a two-tap confirm gate. While physically
verifying it, the device owner did exactly this by mistake -- armed
and confirmed the row for their own working management key
(`home-server-reconnect`), believing it was a disposable test entry,
because nothing on the row distinguished "your current key" from any
other. Recovered in under a minute via the unrelated, already-built
remote-pairing flow (ADR-074 on that branch), but ADR-151 followed
immediately after to make that specific mistake structurally
impossible: the key currently authenticating the connection managing
this device can never be armed or revoked from this screen at all,
regardless of tap count.

This branch (`feat/som-v1`) diverged from `feat/pixel7-native-saaios`
at the same ancestor before either of those two ADRs, and never
independently developed an equivalent -- `handle_trusted_client_tap`
here was, until this ADR, byte-identical to the vulnerable pre-fix
code on the other branch. This ADR ports both fixes forward.

## Decision

**Two-tap arm/confirm**, unchanged from the original: `trusted_client_
revoke_decision(pending: Option<usize>, tapped: usize) -> (Option
<usize>, bool)`, a pure, tested function. First tap on a row arms it;
a second tap on the *same* index confirms and only then does
`revoke_trusted_client` actually run; a tap on any *other* index
re-arms that one instead of revoking the previously-armed row. New
state field `pending_revoke_trusted_client: Option<usize>`, cleared on
opening the screen fresh and on `Back`.

**Never-revocable current-session key**, also unchanged in logic:
`recently_authenticated_key_fingerprints()` reads the last 64 KiB
(never the whole file -- `/run/dropbear.log` is unbounded and
append-only, already well past 14 MB on the branch this was first
built on) of `/run/dropbear.log` for any fingerprint with a recent
"Pubkey auth succeeded" line. `trusted_client_is_protected(fingerprint,
recent) -> bool` is checked in `handle_trusted_client_tap` *before*
`pending_revoke_trusted_client` is touched at all -- a protected row
can never arm and never confirm.

**Adapted for this branch's own rendering path**: `feat/pixel7-native-
saaios` built card rects directly (`stacked_row_rect`); this branch
paints trusted-client rows through `layout_live_v2`/`list_paint_cards`
(ADR-226 here). `trusted_client_card_from_row` still gained the same
`armed`/`protected` parameters and the same card-content logic
(protected: "Ключ этой сессии — нельзя отозвать", no action; armed:
"Нажмите ещё раз, чтобы отозвать" / "Отозвать?", `.selected(true)`) --
only the call site building the `cards` vector changed, from a bare
`.map(trusted_client_card_from_row)` to an `.enumerate().map(...)`
computing `armed`/`protected` per index, matching this branch's own
existing `recent_fingerprints` computation point.

**Not ported**: ADR-152 (a body-free "essential attention" dot on the
no-PIN lock screen) was found, while comparing the two branches for
this port, to already exist here independently (commit `6ce30c0`,
2026-09-19, three days before the other branch's version) --and this
branch's own version is more complete (it also handles the
store-disconnected/offline case, which the other branch's version does
not, and renders through the existing `StatusIndicator` component
rather than a bare color rect). Nothing to port; the other branch's
ADR-152 will be treated as superseded once this comparison is shared
back.

## Verification

- `cargo test -p saai-shell`: 314/314 (adds the same 6 tests ADR-150/
  151 introduced: `trusted_client_revoke_needs_a_second_tap_on_the_
  same_row`, `trusted_client_revoke_confirms_on_the_matching_second_
  tap`, `trusted_client_revoke_a_different_row_rearms_instead_of_
  revoking`, `armed_trusted_client_card_reads_as_a_confirm_prompt`,
  `protected_trusted_client_card_has_no_action_regardless_of_armed`,
  `trusted_client_is_protected_matches_only_a_recent_fingerprint`).
- `cargo build --workspace`: clean exit, no new warnings traceable to
  this change (checked the full warning list; all ~49 pre-existing
  `saai-shell` warnings are unrelated dead-code from this branch's own
  in-progress `sui-v2` migration, none name any symbol this ADR added).
- `cargo clippy --workspace --all-targets -- -D warnings` **could not
  be used to verify this specific change** -- it currently fails
  *before* reaching `saai-shell` at all, on pre-existing, unrelated
  issues in `crates/saai-ui-core` (a `manual_flatten` lint, fixed in
  the same pass since it was trivial and blocking, see the sibling
  commit) and `crates/policy-engine` (a `too_many_arguments` lint on
  `decide_scoped`, **left as found** -- unrelated to this ADR, this
  branch's own active work, not touched here). This is a pre-existing
  gap on this branch, not introduced by this ADR; flagged rather than
  silently worked around.
- Not yet physically confirmed on Pixel 7 -- this branch has not been
  flashed as part of this port; needs its own build/deploy cycle
  before the two-tap gate and the current-session protection can be
  verified against a real device the way ADR-150/151 originally were.

## Consequences

- This branch's trusted-clients screen now has the same safety
  properties `feat/pixel7-native-saaios` already verified physically
  (including surviving a real accidental-revoke incident during that
  verification): no single-tap revoke, and the key managing the
  current session can never be revoked from here at all.
- `crates/saai-ui-core`'s `manual_flatten` clippy debt is fixed as an
  incidental side effect of trying to verify this change; `crates/
  policy-engine`'s `too_many_arguments` debt (and any other pre-
  existing clippy debt elsewhere in the workspace not yet discovered)
  remains open, this branch's own concern going forward.
- `feat/pixel7-native-saaios`'s own ADR-152 (lock-screen attention) is
  superseded by this branch's pre-existing, more complete `6ce30c0` --
  worth confirming explicitly when the two branches' histories are
  next discussed, so nobody re-does this a third time.

## Rollback

Purely additive state/branching logic plus one unrelated one-line
clippy fix -- no data or protocol changes. Revert this commit (and the
sibling `manual_flatten` fix, if desired) to restore the prior
behavior on this branch.

## Links

- `feat/pixel7-native-saaios`'s own ADR-150, ADR-151, ADR-074 (the
  remote-pairing recovery flow, unrelated to and unmodified by this
  port) -- the original design, incident, and fix this ADR carries
  forward.
- `feat/pixel7-native-saaios`'s own ADR-152, superseded by this
  branch's pre-existing `6ce30c0`, per this ADR's own Decision section.
