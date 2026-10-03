#!/usr/bin/env bash
# Make stale branches read-only (GitHub "lock branch"). Nothing is deleted
# and the lock is reversible. Needs a token with repo admin rights.
#
#   .github/scripts/lock-stale-branches.sh            # dry run, prints plan
#   .github/scripts/lock-stale-branches.sh --apply    # actually lock
#   .github/scripts/lock-stale-branches.sh --unlock   # undo for the same list
#
# The list and the evidence are in docs/os/BRANCHES.md.
set -euo pipefail

repo="${REPO:-Damasker/saaios}"
mode="plan"
case "${1:-}" in
  --apply) mode="lock" ;;
  --unlock) mode="unlock" ;;
  "") ;;
  *) echo "usage: $0 [--apply|--unlock]" >&2; exit 2 ;;
esac

stale=(
  cursor/chore-ignore-dist-945d
  cursor/chore-ignore-memory-945d
  cursor/platform-0.5-appliance-945d
  cursor/platform-0.5-streaming-945d
  cursor/platform-0.6-gui-945d
  cursor/platform-0.6-stage-polish-945d
  cursor/platform-0.6-token-stream-945d
  phase-d-poweroff-gesture
  feat/s02-s03-host-slice
  feat/s02-wayland-host
  feat/s03-pixel7-displayd
  feat/vui-04-navigation
  rescue/saaios-a2
  rescue/saaios-a3
  rescue/saaios-a5
  rescue/saaios-b1
  rescue/saaios-b2
  rescue/saaios-vui06
  rescue/saaios-vui06-flash
  rescue/saaios-vui07
)

lock_body() {
  printf '{"required_status_checks":null,"enforce_admins":true,"required_pull_request_reviews":null,"restrictions":null,"lock_branch":%s,"allow_force_pushes":false,"allow_deletions":false}' "$1"
}

for branch in "${stale[@]}"; do
  case "$mode" in
    plan) echo "would lock   $branch" ;;
    lock)
      lock_body true | gh api -X PUT "repos/$repo/branches/$branch/protection" --input - >/dev/null
      echo "locked       $branch" ;;
    unlock)
      lock_body false | gh api -X PUT "repos/$repo/branches/$branch/protection" --input - >/dev/null
      echo "unlocked     $branch" ;;
  esac
done
