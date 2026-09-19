# ADR-150: VUI-07 -- trusted-client revoke needs a confirming second tap

## Status

Accepted, 2026-09-19. Host-verified (4 new tests, full workspace test
+ clippy clean) and physically confirmed on Pixel 7 -- see Verification,
including a real incident this confirmation gate was specifically built
to prevent, and recovered from cleanly via the existing remote-pairing
flow (ADR-074).

## Context

VUI-07's "Apply shared empty, loading, offline, blocked, failed,
permission, confirmation, and recovery patterns" task. ADR-114 already
did this audit for the composed `Сейчас` screen; this pass audited
every other screen ADR-127 through ADR-149 migrated, looking for the
same class of gap (a state silently missing or a signal computed but
never shown).

The most severe real gap found: `handle_trusted_client_tap`'s
`TrustedClientTap::Revoke(index)` arm called `revoke_trusted_client
(index)` on a single tap, unconditionally -- and `trusted_client_
action_at` makes the *entire row* the tap target, not a small button
inside it (`stacked_row_rect(index, ...)`, the whole card). Revoking a
trusted SSH client is irreversible (rewrites `authorized_keys` on the
spot) and security-relevant in a specific, sharp way this screen's own
users are most exposed to: an admin reaching this device over SSH right
now, scrolling this exact list, could revoke the very key carrying that
session with one mis-aimed tap. Every other genuinely dangerous action
in this codebase already gets a real confirmation step of some kind --
`process_dangerous_intent`/the OAM `requires_confirmation` gate on the
`saai-taskd` side, PIN setup's own Отмена/Готово two-step on this same
screen's sibling -- this one row-tap action didn't.

Two lower-severity gaps from the same audit pass, deliberately not
fixed here (see Consequences):

- **Wi-Fi list conflates "no networks" with "wpa_supplicant
  unreachable"** -- `wifi_list_rows` takes no connectivity flag;
  `wifi_scan_results()` silently returns an empty `Vec` on any `wpa_cli`
  failure. Same bug class ADR-114 fixed for `Сейчас`'s whole-screen
  empty state, not yet fixed here.
- **Bluetooth pairing failure is computed but never drawn** --
  `bluetooth_pair_result()`/`bluetooth_status_summary()` parse a real
  `PAIR-ERROR` line into Russian text, but have zero callers in the live
  draw path. Same shape as ADR-114's own second finding
  (`ContextHeader.lifecycle` computed but never rendered).

## Decision

**Two-tap arm/confirm**, not a modal or a separate confirm screen --
this codebase already has a precedent for exactly this shape (`SpaceRow`:
tap selects, retap on the *same* space cycles lifecycle, ADR-128), so
this reuses it rather than inventing a third confirmation pattern:

- `trusted_client_revoke_decision(pending: Option<usize>, tapped:
  usize) -> (Option<usize>, bool)`: pure, tested function. Tapping an
  unarmed row arms it (`(Some(tapped), false)`) without touching the
  file. Tapping the *same* armed row confirms (`(None, true)`) and only
  then does `handle_trusted_client_tap` call `revoke_trusted_client`.
  Tapping any *other* row re-arms that one instead of revoking the
  previously-armed row -- moving your finger to a different row must
  never be read as "yes, revoke the first one."
- New state field `pending_revoke_trusted_client: Option<usize>`,
  cleared on opening the screen fresh (`"open_trusted_clients"`) and on
  `Back` -- leaving cancels, matching every other modal-ish screen in
  this file.
- `trusted_client_card_from_row` gained an `armed: bool` parameter: an
  armed row's status text becomes "Нажмите ещё раз, чтобы отозвать"
  (replacing the fingerprint, which would crowd this row's own width
  alongside the warning) and its action label becomes "Отозвать?"; the
  card is also marked `.selected(true)` for the existing elevated-
  background treatment `ActionCardView` already has, so an armed row is
  visually distinct at a glance, not just on close reading.

**Not done, deliberately, as part of this ADR**: the Wi-Fi and
Bluetooth gaps above are real but are a different failure shape
(missing signal, not missing confirmation) and touch different call
sites (`wifi_list_rows`/`wifi_scan_results`, `bluetooth_pair_result`/
the pairing tap handler) -- bundling three unrelated fixes under one
ADR would make each harder to verify and roll back independently.
Recorded here, not silently dropped, same as ADR-114's own "deliberately
not built, flagged" section.

## Verification

- `cargo test -p saai-shell`: 195/195 (191 + 4 new --
  `trusted_client_revoke_needs_a_second_tap_on_the_same_row`,
  `trusted_client_revoke_confirms_on_the_matching_second_tap`,
  `trusted_client_revoke_a_different_row_rearms_instead_of_revoking`,
  `armed_trusted_client_card_reads_as_a_confirm_prompt`). The existing
  `trusted_client_list_rows_use_name_and_fingerprint_prefix` test
  updated for `trusted_client_card_from_row`'s new `armed` parameter,
  no assertions weakened.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: unchanged pass count elsewhere (77
  test-result blocks, all `ok`).
- `handle_trusted_client_tap` itself (like every sibling `handle_*_tap`
  method in this file) needs a real `Connection`/`QueueHandle` to call
  and has no unit tests of its own -- the arm/confirm *decision* was
  deliberately factored into the pure, directly-tested
  `trusted_client_revoke_decision` function above specifically so this
  logic wouldn't be untestable by construction.
- **Physically confirmed** (2026-09-19), including a real incident:
  the device owner tapped a row once, saw the arm warning render
  correctly, tapped Back to cancel per the first ask -- confirming the
  gate holds on a single tap and leaving does cancel. On a second,
  separate check intended to exercise a disposable test entry, the
  device owner tapped a row twice (arm, then confirm) believing it was
  a spare test key; it was actually `home-server-reconnect`, this
  session's own working SSH key, and the second tap revoked it exactly
  as designed -- the gate did its job (a mis-identified row, not an
  accidental single tap, is what caused this), but it is worth naming
  plainly: two rows in this list currently start with the identical
  `home-server-` prefix (`home-server-test`, `home-server-reconnect`),
  and nothing besides the trailing name distinguishes them at a glance.
  Recovery used the existing, already-built remote-pairing flow
  (ADR-074): a fresh `PAIR home-server-reconnect\n<pubkey>\n` request
  to `pair-recv` on port 7779 showed the on-device consent screen, the
  device owner tapped Разрешить, and SSH access was restored within
  the same minute, `authorized_keys` back to its expected three
  entries. No data lost, no rollback needed -- but this is the second
  time in this same ADR that "which row is which" mattered more than
  the tap count. See Consequences for the follow-up this suggests.

## Consequences

- Revoking a trusted SSH client on this screen now needs two
  deliberate taps on the same row -- a single mis-tap while scrolling
  can no longer cut off a live admin session.
- **New follow-up finding, from the physical confirmation itself**: the
  arm/confirm gate stops an accidental single tap, but does nothing
  about confidently tapping the *wrong* row on purpose -- confirmed
  live when the device owner armed-then-confirmed `home-server-
  reconnect` while intending a same-prefixed test entry. `TrustedClient`
  only carries `client_name`/`fingerprint`; nothing about "is this the
  connection you're using right now" is visible. Worth its own pass
  (not this ADR -- a different failure shape again, per this ADR's own
  Context on why the Wi-Fi/Bluetooth findings weren't bundled in
  either): e.g. surfacing which key authenticated the *current* SSH
  session, if that is knowable from `dropbear`, so that row can be
  marked and never silently revocable at all.
- The Wi-Fi offline/empty conflation and the invisible Bluetooth
  pairing failure remain open findings from this same audit pass,
  intentionally left for a following ADR rather than bundled in here.
- `PinSetupState`'s "Убрать PIN" action (main.rs, same one-tap-no-
  confirm shape, lower severity since it's the device's own local lock
  rather than remote access) was noted by this audit but not fixed
  here -- worth the same treatment next time that screen is touched.

## Rollback

`saai-shell.pre-tcconfirm` remains on-device once deployed. Purely
additive state/branching -- no data or protocol changes, and
`authorized_keys`'s own format is untouched.

## Links

- ADR-114 -- the audit methodology and bug class (a real signal that
  exists in code but never reaches the user) this ADR's first two
  "not fixed here" findings both match.
- ADR-128 -- `SpaceRow`'s retap-cycles-lifecycle gesture, the existing
  precedent this ADR's arm/confirm shape reuses rather than inventing
  a new one.
