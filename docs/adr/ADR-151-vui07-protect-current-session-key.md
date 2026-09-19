# ADR-151: VUI-07 -- never let the trusted-clients screen revoke the key managing this device right now

## Status

Accepted, 2026-09-19. Host-verified only (2 new tests, full workspace
test + clippy clean). Not yet physically re-confirmed on Pixel 7 -- see
Verification.

## Context

Direct follow-up to a real incident ADR-150 recorded while its own
two-tap confirm gate was being physically checked: the device owner
armed-then-confirmed `home-server-reconnect` (this session's own
working management key) while intending a same-prefixed test entry
(`home-server-test`). ADR-150's gate did exactly what it was built for
-- it stopped an accidental single tap -- but did nothing against a
confident, deliberate two-tap sequence on the wrong row. Recovery used
the existing remote-pairing flow (ADR-074) and took under a minute with
no data lost, but the right fix is to make the mistake structurally
impossible for the one row that matters most: whichever key is actually
managing this device right now should never be revocable from this
screen at all, confirm gate or not.

`TrustedClient` only carries `client_name`/`fingerprint` -- nothing
about "is this the connection you're using right now" is visible or
even computed anywhere.

## Decision

**Recency proxy, not a live-session check.** Dropbear's own log
(`/run/dropbear.log`) records a `Pubkey auth succeeded ... key
SHA256:...` line per authenticated connection, so in principle "is
there a currently-open session for this key" is answerable from
"Exit" lines too. Not used: this project's own SSH usage (this
session's included) is almost entirely quick single-command
connections that open, run one command, and close within the same
second -- a strict "session still open" check would almost never be
true for the key that just, functionally, revoked itself a minute
earlier. Used instead: `recently_authenticated_key_fingerprints()`
collects every fingerprint with a successful-auth line anywhere in the
log's **last 64 KiB** (not the whole file -- by the time this was
written, one work session had already grown this unbounded, unrotated
log past 14 MB / 147000 lines; reading it in full on every trusted-
clients draw would be a real, easily-hit cost the 64 KiB tail avoids
while still covering comfortably more than enough recent connections
for any realistic cadence).

**Protection is enforced at the handler, not just the render.**
`trusted_client_is_protected(fingerprint, recent) -> bool`, a pure
function kept separate from both the log-reading function above and
ADR-150's own `trusted_client_revoke_decision`, so each stays
independently testable. `handle_trusted_client_tap`'s `Revoke(index)`
arm now checks this **before** touching `pending_revoke_trusted_client`
at all -- a protected row can never arm and can never confirm, no
matter how many times or how deliberately it's tapped. This is
stronger than ADR-150's gate on purpose: that gate assumes a real
confirm should eventually be honored; this one says a specific row
must never be revocable through this screen at all while it looks
recently active.

**Rendering matches**: `trusted_client_card_from_row` gained a
`protected: bool` parameter, checked before `armed` (a row that can
never be revoked must never render "Нажмите ещё раз, чтобы отозвать" --
protected wins even in the (currently unreachable, since the handler
never arms a protected row) case where both would otherwise be true).
A protected row shows "Ключ этой сессии — нельзя отозвать" with no
action label at all, the same "nothing to tap" convention this
screen's own offline/empty rows already use.

**Not done**: an actually precise "is this exact TCP connection still
open" check via dropbear's Exit lines, which would need per-PID
correlation across the log and would still miss the real failure mode
this ADR fixes (a *recently* used, not *currently* connected, key).
Also not done: log rotation for `/run/dropbear.log` itself -- a real,
separate operational concern this ADR's own read noticed but does not
fix (different failure shape again, same reasoning ADR-150 gave for not
bundling its own Wi-Fi/Bluetooth findings in).

## Verification

- `cargo test -p saai-shell`: 197/197 (195 + 2 new --
  `protected_trusted_client_card_has_no_action_regardless_of_armed`,
  `trusted_client_is_protected_matches_only_a_recent_fingerprint`).
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: unchanged pass count elsewhere (77
  test-result blocks, all `ok`).
- `recently_authenticated_key_fingerprints`/the actual protection
  check inside `handle_trusted_client_tap` are not unit-tested directly
  (the former does real file I/O against a live path that doesn't
  exist on a host test machine, returning an empty `Vec` there by its
  own documented fail-safe behavior; the latter needs a real
  `Connection`/`QueueHandle`, same as every sibling `handle_*_tap`
  method) -- the decision logic itself (`trusted_client_is_protected`)
  is what's tested, matching the exact precedent ADR-150 already set
  for `trusted_client_revoke_decision`.
- Not yet physically re-confirmed: needs an actual on-device check
  that the currently-connected management key's row now renders
  non-actionable and a tap on it does nothing, ideally performed while
  a real SSH connection from `home-server` is active so the 64 KiB tail
  window is guaranteed to contain a fresh auth line.

## Consequences

- The exact incident ADR-150 recorded cannot recur through this
  screen: the key that just authenticated the connection driving this
  work is never revocable from here, regardless of tap count or intent.
- A key that hasn't been used in a while (older than the log's 64 KiB
  tail comfortably reaches back) reverts to ADR-150's ordinary two-tap
  confirm -- this ADR narrows, rather than replaces, that gate.
- `/run/dropbear.log`'s unbounded growth (14 MB and climbing, no
  rotation) is now a load-bearing fact for this feature's own
  performance, not just background noise -- worth its own fix someday,
  flagged here rather than silently relied upon.

## Rollback

`saai-shell.pre-protectkey` remains on-device once deployed. Purely
additive read/branching logic -- no data or protocol changes, and a
rollback simply restores ADR-150's plain two-tap behavior for every
row including the currently-active one.

## Links

- ADR-150 -- the two-tap confirm gate this ADR narrows further, and
  the real incident that motivated this one directly.
- ADR-074 -- the remote-pairing flow that recovered from that incident
  cleanly, independent of and unmodified by this ADR.
