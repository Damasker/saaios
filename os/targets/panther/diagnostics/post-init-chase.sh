#!/bin/sh
# After an external catalog OEM SIM_INIT (0x2f50) frame is injected (or already
# applied), poll GET_APP and run the proven post-edge bearer chase once READY
# / DETECTED / PERSO (app in {1,4,5}).
#
# Does NOT invent 0x2f50 bytes. Does NOT start cbd/rild.
# Soft-lock still needs an external frame for bearer — this only arms chase.
#
# Usage:
#   FRAME_FILE=/path/to/capture.bin sh post-init-chase.sh
#   sh post-init-chase.sh --frame /path/to/capture.hex [--oem N]
#   sh post-init-chase.sh --poll-only   # skip inject; poll then chase
#
# Env:
#   SIT, BIN, OUT, POLL_SECS (default 90), POLL_INTERVAL (default 2)
#   APN_FILE, VERIFY_TOOL — same as tray-bearer-chase
set -eu

SIT=${SIT:-/data/saaios/bin/sit-sim-status}
BIN=${BIN:-/data/saaios/bin}
OUT=${OUT:-/data/saaios/var/post-init-chase.log}
POLL_SECS=${POLL_SECS:-90}
POLL_INTERVAL=${POLL_INTERVAL:-2}
APN_FILE=${APN_FILE:-/data/saaios/etc/apn}
CHASE=${CHASE:-}
FRAME_FILE=${FRAME_FILE:-}
OEM_N=${OEM_N:-0}
POLL_ONLY=0
INJECT=${INJECT:-}

mkdir -p "$(dirname "$OUT")" /run /data/saaios/etc
{
  printf '=== post-init-chase start %s ===\n' "$(date -Iseconds 2>/dev/null || date)"
} >> "$OUT"
log() { printf '%s\n' "$*" | tee -a "$OUT"; }

resolve_tool() {
  name=$1
  if [ -n "${2:-}" ] && [ -x "$2" ]; then
    printf '%s\n' "$2"
    return 0
  fi
  if [ -x "/tmp/$name" ]; then
    printf '%s\n' "/tmp/$name"
    return 0
  fi
  if [ -x "$BIN/$name" ]; then
    printf '%s\n' "$BIN/$name"
    return 0
  fi
  # Repo-relative when running from a checkout on-device/host.
  HERE=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
  if [ -x "$HERE/$name" ]; then
    printf '%s\n' "$HERE/$name"
    return 0
  fi
  return 1
}

usage() {
  cat <<'EOF'
usage: post-init-chase.sh [--frame FILE] [--oem N] [--poll-only]
  --frame FILE   inject once via oem-ipc-inject (raw or hex; no invent)
  --oem N        oem_ipcN 0..7 (default 0)
  --poll-only    skip inject; poll GET_APP then chase-once
Env FRAME_FILE / OEM_N / POLL_SECS also accepted.
Never starts rild/cbd. Never invents 0x2f50.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --frame)
      [ "$#" -ge 2 ] || { usage; exit 64; }
      FRAME_FILE=$2
      shift 2
      ;;
    --oem)
      [ "$#" -ge 2 ] || { usage; exit 64; }
      OEM_N=$2
      shift 2
      ;;
    --poll-only)
      POLL_ONLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      log "unknown arg: $1"
      usage
      exit 64
      ;;
  esac
done

STATE=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
log "STATE=$STATE"
[ "$STATE" = ONLINE ] || { log "need ONLINE"; exit 1; }

INJECT=$(resolve_tool oem-ipc-inject "${INJECT:-}" || true)
if [ -z "$CHASE" ]; then
  CHASE=$(resolve_tool tray-bearer-chase.sh || true)
  if [ -z "$CHASE" ]; then
    HERE=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
    if [ -r "$HERE/tray-bearer-chase.sh" ]; then
      CHASE="$HERE/tray-bearer-chase.sh"
    fi
  fi
fi

sim_field() {
  echo "$2" | tr ' ' '\n' | grep -E "^${1}=" | head -1 | cut -d= -f2
}

app_start_network_ok() {
  case "${1:-}" in
    1|4|5) return 0 ;;
    *) return 1 ;;
  esac
}

snapshot_sim() {
  set +e
  SIM=$("$SIT" query-sim-status 2>/dev/null | tr '\n' ' ')
  RC=$?
  set -e
  APP=$(sim_field app0_state_raw "$SIM")
  PIN1=$(sim_field pin1_state_raw "$SIM")
  REM=$(sim_field pin1_remain_raw "$SIM")
  CARD=$(sim_field card_state_raw "$SIM")
  log "SIM snapshot app=${APP:-?} pin1=${PIN1:-?} remain=${REM:-?} card=${CARD:-?} rc=$RC"
}

if [ "$POLL_ONLY" != "1" ]; then
  if [ -z "${FRAME_FILE:-}" ]; then
    log "ABORT need --frame FILE or FRAME_FILE=... (or --poll-only)"
    log "HINT soft-lock still needs external catalog 0x2f50 frame; see OEM-IPC-CAPTURE.md"
    exit 2
  fi
  if [ ! -r "$FRAME_FILE" ]; then
    log "ABORT frame not readable path_set=1 (path not logged)"
    exit 2
  fi
  if [ -z "${INJECT:-}" ] || [ ! -x "$INJECT" ]; then
    log "ABORT oem-ipc-inject missing (build+deploy diagnostics/oem-ipc-inject.c)"
    exit 1
  fi
  log "inject via $INJECT oem_ipc$OEM_N (frame body not logged)"
  INJECT_LOG=/tmp/oem-ipc-inject.$$.log
  set +e
  "$INJECT" inject "$FRAME_FILE" "$OEM_N" >"$INJECT_LOG" 2>&1
  IRC=$?
  set -e
  tee -a "$OUT" <"$INJECT_LOG" || true
  rm -f "$INJECT_LOG"
  if [ "$IRC" -ne 0 ]; then
    log "ABORT inject rc=$IRC"
    exit 3
  fi
  log "inject ok — polling GET_APP for app in {1,4,5}"
else
  log "poll-only — no inject"
fi

ELAPSED=0
READY=0
while [ "$ELAPSED" -le "$POLL_SECS" ]; do
  ST=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
  if [ "$ST" != ONLINE ]; then
    log "ABORT STATE=$ST during poll"
    exit 1
  fi
  snapshot_sim
  if app_start_network_ok "${APP:-}"; then
    log "GET_APP ok app=${APP} — starting chase-once"
    READY=1
    break
  fi
  # Also accept textual READY if raw missing.
  if echo "${SIM:-}" | grep -qE 'app0_state=READY|app0_state_raw=5'; then
    log "GET_APP READY — starting chase-once"
    READY=1
    break
  fi
  sleep "$POLL_INTERVAL"
  ELAPSED=$((ELAPSED + POLL_INTERVAL))
done

if [ "$READY" -ne 1 ]; then
  log "TIMEOUT poll_secs=$POLL_SECS app still not in {1,4,5}"
  log "soft-lock holds — bearer needs evidenced catalog 0x2f50 frame (not invented)"
  exit 4
fi

if [ -z "${CHASE:-}" ] || [ ! -r "$CHASE" ]; then
  log "ABORT tray-bearer-chase.sh missing"
  exit 1
fi

log "CHASE_ONCE via $CHASE"
set +e
env CHASE_ONCE=1 SIT="$SIT" BIN="$BIN" OUT="$OUT" APN_FILE="$APN_FILE" \
  sh "$CHASE"
CRC=$?
set -e
if [ "$CRC" -eq 0 ]; then
  log "GOAL bearer OK (post-init-chase)"
  exit 0
fi
log "chase-once finished rc=$CRC (bearer not verified)"
exit "$CRC"
