#!/bin/sh
# Watch a SIM edge, then run the guarded post-READY network diagnostic.
# Never logs PIN digits or AID. Stop if remain<=1. No POWER_OFF / crash.
#
# Observes these published PIN shapes:
#   app=PIN + pin1=DISABLED(3)
#   app=PIN + pin1=ENABLED_VERIFIED(2)  (Pin1Verified OK; still no START_NETWORK)
#   app=PIN + pin1=NOT_VERIFIED(1) (VerifyPin requires explicit opt-in)
#
# Tested tray reseat and passive wait left app at PIN. A concurrent diagnostic
# twice reached READY from PIN/pin1=1 using VerifyPin A+AID without CardPower.
# It held SIT during the RFS 7->3->6 test; that test cannot prove self-init.
# Current CP Present is not on SIT wire.
# CPIF caps (AP part0=3 / CP part0=7) were exercised at INIT_START.
#
# Once a fresh SIM status confirms READY(5) and radio ON(10):
#   guarded ready-network-once (selection auto if needed + AllowData once)
#   -> data-reg 1/5 -> optional SetupDataCall(if APN) -> rmnet IPv4/rx+tx
#
# WATCH_ROUNDS: each tray-watch is ~180s; default 12 (~36 min).
# PERSIST=1: when rounds expire without bearer, re-arm another batch
#   (same flock); later state changes can trigger chase. Exit only on
#   BEARER_OK or fatal OFFLINE/ABORT.
# VERIFY_TOOL: explicitly chosen binary; never selected automatically.
# ALLOW_PIN_VERIFY=1: explicit opt-in; default 0 never attempts PIN.
# ALLOW_SETUP_DATA_CALL=1: explicit opt-in after registration; default 0.
# APN_FILE: /data/saaios/etc/apn (operator-supplied host; never invent).
set -eu
SIT=${SIT:-/data/saaios/bin/sit-sim-status}
BIN=${BIN:-/data/saaios/bin}
OUT=${OUT:-/data/saaios/var/tray-bearer.log}
WATCH_ROUNDS=${WATCH_ROUNDS:-12}
BEARER_POLLS=${BEARER_POLLS:-60}
PERSIST=${PERSIST:-1}
APN_FILE=${APN_FILE:-/data/saaios/etc/apn}
ALLOW_PIN_VERIFY=${ALLOW_PIN_VERIFY:-0}
[ "$ALLOW_PIN_VERIFY" = "1" ] || ALLOW_PIN_VERIFY=0
ALLOW_SETUP_DATA_CALL=${ALLOW_SETUP_DATA_CALL:-0}
[ "$ALLOW_SETUP_DATA_CALL" = "1" ] || ALLOW_SETUP_DATA_CALL=0
PIN_VERIFY_ATTEMPTED=0
mkdir -p "$(dirname "$OUT")" /run /data/saaios/etc
{
  printf '=== tray-bearer-chase start %s rounds=%s persist=%s ===\n' "$(date -Iseconds 2>/dev/null || date)" "$WATCH_ROUNDS" "$PERSIST"
} >> "$OUT"
log() { printf '%s\n' "$*" | tee -a "$OUT"; }

# Status snapshot for soft-lock hold (no secrets; matches saai-modemd soft-lock).
log_soft_lock_status() {
  log "SOFT_LOCK_STATUS modem06=yes cpif_caps_exercised=yes cpif_caps_note=AP_part0=3(PKTPROC_UL|CH_EXT)_CP_part0=7(+36BIT) blocker=cp_app_state_pin_blocks_start_network present_status=unknown_not_on_sit_wire panther_observed=verify_pin_a_aid_without_cardpower_ready_twice rfs_causality=unproven_concurrent_verify_pin_sit_owner goal=incomplete_until_rmnet_ipv4 ${1:-}"
}

# Resolve helper: prefer /tmp overlay, else /data/saaios/bin (deployed stock).
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
  return 1
}

VERIFY_TOOL=${VERIFY_TOOL:-}
# No default verification tool: the proven pin1=1 path skips CardPower.
# ALLOW_PIN_VERIFY=1 also requires an explicitly selected binary.
SETUP_DC=$(resolve_tool setup-data-call || true)
READY_NETWORK=${READY_NETWORK:-$BIN/ready-network-once}

# Refuse concurrent chases via flock (ps cmdline matches self under nohup/env).
CHASE_LOCK=${CHASE_LOCK:-/run/saaios-tray-chase.lock}
exec 9>"$CHASE_LOCK"
if ! flock -n 9; then
  log "ABORT another tray-bearer-chase holds $CHASE_LOCK"
  exit 1
fi

STATE=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
log "STATE=$STATE"
[ "$STATE" = ONLINE ] || { log "need ONLINE"; exit 1; }

log "TOOLS verify=${VERIFY_TOOL:-none} pin_verify_opt_in=$ALLOW_PIN_VERIFY network=${READY_NETWORK:-none} setupdc=${SETUP_DC:-none} setupdc_opt_in=$ALLOW_SETUP_DATA_CALL persist=$PERSIST"
if [ -r "$APN_FILE" ]; then
  log "APN_FILE present path=$APN_FILE (contents not logged)"
else
  log "APN_FILE missing path=$APN_FILE - SetupDataCall deferred until operator writes carrier APN"
fi

# Document signed CPIF capabilities already negotiated at INIT_START (RO).
if [ -r /sys/devices/platform/cpif/info_region ]; then
  CAP=$(tr '\n' ' ' </sys/devices/platform/cpif/info_region | sed 's/  */ /g')
  # Non-secret offsets only; strip nothing that looks like identity.
  log "CPIF_CAPS $(echo "$CAP" | sed -n 's/.*\(ap_capability_offset\[0\]:[^ ]*\).*/\1/p') $(echo "$CAP" | sed -n 's/.*\(cp_capability_offset\[0\]:[^ ]*\).*/\1/p') $(echo "$CAP" | sed -n 's/.*\(ap_capability_offset\[1\]:[^ ]*\).*/\1/p') $(echo "$CAP" | sed -n 's/.*\(cp_capability_offset\[1\]:[^ ]*\).*/\1/p')"
  log "CPIF_CAPS_NOTE AP=3 (PKTPROC_UL|CH_EXT) CP=7 (+36BIT) negotiated at INIT_START; not a READY lever"
fi

# Parse non-secret fields only from query-sim-status text.
sim_field() {
  # usage: sim_field "pin1_state_raw" "$SIM_LINE"
  echo "$2" | tr ' ' '\n' | grep -E "^${1}=" | head -1 | cut -d= -f2
}

# Decisive falsifier line on EDGE/ABSENT/PRESENT (no secrets).
# post_edge=yes means unlock+chase will run; no means soft-lock hold.
log_edge_decision() {
  # usage: log_edge_decision <tag> <app> <pin1> <card> <present_status> <post_edge_yes_no> [extra]
  log "EDGE_DECISION tag=$1 app=${2:-?} pin1=${3:-?} card=${4:-?} present_status=${5:-unknown} post_edge=$6 ${7:-}"
}

remain_ok() {
  R=$(sim_field pin1_remain_raw "$1")
  [ -n "$R" ] || return 1
  [ "$R" -gt 1 ] 2>/dev/null
}

# START_NETWORK / camp gate (CP GET_APP CMP #1/#4/#5).
app_start_network_ok() {
  case "${1:-}" in
    1|4|5) return 0 ;;
    *) return 1 ;;
  esac
}

# After a fresh PIN state: CardPower+VerifyPin requires explicit opt-in and
# a measured pin1=0|1 with more than one attempt left.
# Logs only app/pin1/remain/error - never PIN/AID bytes.
unlock_after_edge() {
  log "=== phase1b Pin1Verified attempt (no secrets) ==="
  set +e
  RAW_SIM=$("$SIT" query-sim-status 2>/dev/null)
  SIM_RC=$?
  set -e
  SIM=$(printf '%s' "$RAW_SIM" | tr '\n' ' ')
  APP=$(sim_field app0_state_raw "$SIM")
  PIN1=$(sim_field pin1_state_raw "$SIM")
  REM=$(sim_field pin1_remain_raw "$SIM")
  CARD=$(sim_field card_state_raw "$SIM")
  log "pre_unlock app_raw=${APP:-?} pin1=${PIN1:-?} remain=${REM:-?} card=${CARD:-?} query_rc=$SIM_RC"

  if [ "$SIM_RC" -ne 0 ] || [ -z "$APP" ] || [ -z "$PIN1" ] || [ "$CARD" != "1" ]; then
    log "STOP SIM query failed, incomplete, or card absent - VerifyPin not sent"
    return 1
  fi
  # Already READY / DETECTED / PERSO - skip verify
  if app_start_network_ok "${APP:-}"; then
    log "app already ${APP} (START_NETWORK set) - skip VerifyPin"
    return 0
  fi
  if [ "$APP" != "2" ]; then
    log "STOP app=$APP is not a PIN verification state"
    return 1
  fi
  case "$PIN1" in
    2|3)
      log "pin1=$PIN1 already verified or disabled - skip CardPower/VerifyPin"
      return 0
      ;;
    0|1)
      if [ "$ALLOW_PIN_VERIFY" != "1" ]; then
        log "STOP PIN verification requires ALLOW_PIN_VERIFY=1 - VerifyPin not sent"
        return 1
      fi
      ;;
    *)
      log "STOP unrecognized pin1=$PIN1 - VerifyPin not sent"
      return 1
      ;;
  esac
  if ! remain_ok "$SIM"; then
    log "STOP remaining PIN attempts missing or <=1 - VerifyPin not sent"
    return 1
  fi
  if [ "$PIN_VERIFY_ATTEMPTED" = "1" ]; then
    log "STOP this watcher already made its one permitted VerifyPin attempt"
    return 1
  fi

  if [ -n "${VERIFY_TOOL:-}" ] && [ -x "$VERIFY_TOOL" ]; then
    log "running explicitly enabled VERIFY_TOOL once"
    PIN_VERIFY_ATTEMPTED=1
    set +e
    "$VERIFY_TOOL" >/dev/null 2>&1
    VERIFY_RC=$?
    set -e
    log "VERIFY_TOOL exit=$VERIFY_RC (output withheld to protect PIN/AID)"
  else
    log "VERIFY_TOOL missing - VerifyPin not sent"
    return 1
  fi

  set +e
  RAW_SIM2=$("$SIT" query-sim-status 2>/dev/null)
  SIM2_RC=$?
  set -e
  SIM2=$(printf '%s' "$RAW_SIM2" | tr '\n' ' ')
  log "post_unlock app_raw=$(sim_field app0_state_raw "$SIM2") pin1=$(sim_field pin1_state_raw "$SIM2") remain=$(sim_field pin1_remain_raw "$SIM2") query_rc=$SIM2_RC"
  return 0
}

chase_bearer() {
  log "=== phase2 bearer chase (post-EDGE pipeline) ==="

  # CP START_NETWORK accepts {1,4,5}; this guarded runner requires READY(5).
  set +e
  RAW_SIM0=$("$SIT" query-sim-status 2>/dev/null)
  SIM0_RC=$?
  set -e
  SIM0=$(printf '%s' "$RAW_SIM0" | tr '\n' ' ')
  APP0=$(sim_field app0_state_raw "$SIM0")
  PIN10=$(sim_field pin1_state_raw "$SIM0")
  CARD0=$(sim_field card_state_raw "$SIM0")
  log "chase_gate app_raw=${APP0:-?} pin1=${PIN10:-?} card=${CARD0:-?} query_rc=$SIM0_RC (START_NETWORK needs 1|4|5)"

  START_OK=0
  if [ "$SIM0_RC" -eq 0 ] && [ "${APP0:-}" = "5" ] && [ "${CARD0:-}" = "1" ]; then
    START_OK=1
    log "chase_gate READY=yes - running guarded network diagnostic"
  else
    log "chase_gate READY=no (app=${APP0:-?}) - guarded runner requires published READY(5)"
    return 1
  fi
  if [ ! -x "$READY_NETWORK" ]; then
    log "guarded ready-network-once missing at $READY_NETWORK - network SETs deferred"
    return 1
  fi
  set +e
  NETWORK_OUT=$("$READY_NETWORK" run 2>&1)
  NETWORK_RC=$?
  set -e
  printf '%s\n' "$NETWORK_OUT" | grep -E '^(preflight|selection|AllowData|poll=|RESULT|ABORT)' | tee -a "$OUT" || true
  log "ready-network-once exit=$NETWORK_RC"
  [ "$NETWORK_RC" -eq 0 ] || return 1

  # Old GET helpers also open RFS with incomplete handling. The guarded
  # runner owns the SIT sequence; leave RFS to its separate broker.
  # SetupDataCall remains deferred until a successful reg=1/5 query below.
  log "SetupDataCall deferred_until_data_registration_1_or_5"
  set -e

  i=0
  while [ "$i" -lt "$BEARER_POLLS" ]; do
    set +e
    REG_RAW=$("$SIT" query-data-registration 2>/dev/null)
    REG_RC=$?
    set -e
    REG=$(printf '%s' "$REG_RAW" | tr '\n' ' ')
    REG_STATE=$(sim_field registration_raw "$REG")
    log "data_registration_raw=${REG_STATE:-?} query_rc=$REG_RC poll=$i"
    REGISTERED=0
    if [ "$REG_RC" -eq 0 ]; then
      case "$REG_STATE" in 1|5) REGISTERED=1 ;; esac
    fi
    # Only home(1) or roaming(5) permits the data-call request.
    if [ "$REGISTERED" -eq 1 ]; then
      # The guarded one-shot already handled selection and AllowData.
      if [ "$START_OK" -eq 1 ]; then
        set +e
        if [ "$ALLOW_SETUP_DATA_CALL" != "1" ]; then
          log "SetupDataCall deferred_explicit_opt_in_required"
        elif [ -n "${SETUP_DC:-}" ] && [ -x "$SETUP_DC" ] && [ -r "$APN_FILE" ]; then
          "$SETUP_DC" --apn-file "$APN_FILE" >/dev/null 2>&1
          SETUP_RC=$?
          log "SetupDataCall attempted after reg=$REG_STATE exit=$SETUP_RC (APN/output withheld)"
        elif [ ! -r "$APN_FILE" ]; then
          log "SetupDataCall deferred_no_apn (write carrier APN to $APN_FILE)"
        else
          log "SetupDataCall tool missing - skip 0x0600"
        fi
        set -e
      fi
    fi
    [ "$REGISTERED" -eq 1 ] && break
    # Also accept early IPv4 / bidirectional rmnet mid-poll.
    for n in 0 1 2 3; do
      IF=rmnet$n
      [ -d "/sys/class/net/$IF" ] || continue
      ADDR=$(ip -4 addr show "$IF" 2>/dev/null | sed -n 's/.*inet \([^ ]*\).*/\1/p' | head -1)
      RX=$(cat /sys/class/net/$IF/statistics/rx_bytes 2>/dev/null || echo 0)
      TX=$(cat /sys/class/net/$IF/statistics/tx_bytes 2>/dev/null || echo 0)
      if [ -n "${ADDR:-}" ]; then
        log "BEARER_OK $IF $ADDR (poll=$i)"
        return 0
      fi
      if [ "${RX:-0}" -gt 0 ] 2>/dev/null && [ "${TX:-0}" -gt 0 ] 2>/dev/null; then
        log "BEARER_OK $IF rx=$RX tx=$TX (no IPv4 yet; poll=$i)"
        return 0
      fi
    done
    i=$((i + 1))
    sleep 2
  done

  for n in 0 1 2 3; do
    IF=rmnet$n
    if [ -d "/sys/class/net/$IF" ]; then
      ip link set "$IF" up 2>/dev/null || true
      # Device image may lack awk; parse inet with sed.
      ADDR=$(ip -4 addr show "$IF" 2>/dev/null | sed -n 's/.*inet \([^ ]*\).*/\1/p' | head -1)
      RX=$(cat /sys/class/net/$IF/statistics/rx_bytes 2>/dev/null || echo 0)
      TX=$(cat /sys/class/net/$IF/statistics/tx_bytes 2>/dev/null || echo 0)
      log "IF $IF addr=${ADDR:-none} rx=$RX tx=$TX"
      if [ -n "${ADDR:-}" ]; then
        log "BEARER_OK $IF $ADDR"
        return 0
      fi
      # Live bearer proof without IPv4: both directions nonzero.
      if [ "${RX:-0}" -gt 0 ] 2>/dev/null && [ "${TX:-0}" -gt 0 ] 2>/dev/null; then
        log "BEARER_OK $IF rx=$RX tx=$TX (no IPv4 yet)"
        return 0
      fi
    fi
  done
  log "no IPv4 on rmnet after chase start_network_ok=$START_OK"
  return 1
}

# Re-emit tray-watch transitions as observations. Only a fresh SIM status
# query below can authorize the bearer chase.
log_transitions_from_round() {
  F=$1
  [ -r "$F" ] || return 0
  # shellcheck disable=SC2162
  while IFS= read -r LINE || [ -n "$LINE" ]; do
    case "$LINE" in
      TRANSITION*)
        APP=$(echo "$LINE" | tr ' ' '\n' | grep -E '^app=' | head -1 | cut -d= -f2 | cut -d'(' -f1)
        PIN1=$(echo "$LINE" | tr ' ' '\n' | grep -E '^pin1=' | head -1 | cut -d= -f2)
        CARD=$(echo "$LINE" | tr ' ' '\n' | grep -E '^card=' | head -1 | cut -d= -f2)
        PI=unknown
        CANDIDATE=no
        if [ "${CARD:-}" = "1" ]; then
          case "${APP:-}" in
            5) CANDIDATE=yes ;;
            2) if [ "$ALLOW_PIN_VERIFY" = "1" ]; then
                 case "${PIN1:-}" in 0|1) CANDIDATE=yes ;; esac
               fi ;;
          esac
        fi
        log_edge_decision "transition" "${APP:-?}" "${PIN1:-?}" "${CARD:-?}" "${PI:-?}" "no" "src=tray-watch candidate=$CANDIDATE"
        ;;
      *ABSENT*PRESENT*|*'ABSENT→PRESENT'*|*'ABSENT-PRESENT'*)
        log_edge_decision "absent_present" "?" "?" "?" "unknown" "no" "src=tray-watch_unconfirmed"
        ;;
    esac
  done < "$F"
}

sim_left_pin() {
  # Use a fresh, complete status response. A tray event alone is not a
  # published app transition, and an empty/failed query is inconclusive.
  set +e
  RAW_SIM=$("$SIT" query-sim-status 2>/dev/null)
  RC=$?
  set -e
  SIM=$(printf '%s' "$RAW_SIM" | tr '\n' ' ')
  # Log non-secret fields only
  APP=$(sim_field app0_state_raw "$SIM")
  PIN1=$(sim_field pin1_state_raw "$SIM")
  REM=$(sim_field pin1_remain_raw "$SIM")
  CARD=$(sim_field card_state_raw "$SIM")
  log "SIM snapshot app=${APP:-?} pin1=${PIN1:-?} remain=${REM:-?} card=${CARD:-?} present_status=unknown query_rc=$RC"
  if [ "$RC" -ne 0 ] || [ -z "$APP" ] || [ -z "$CARD" ]; then
    log_edge_decision "query_inconclusive" "${APP:-?}" "${PIN1:-?}" "${CARD:-?}" "unknown" "no"
    return 1
  fi
  if [ "$CARD" != "1" ]; then
    log_edge_decision "card_not_present" "$APP" "${PIN1:-?}" "$CARD" "unknown" "no"
    return 1
  fi
  case "$APP" in
    5)
      log_edge_decision "app_ready_for_guarded_runner" "$APP" "${PIN1:-?}" "$CARD" "unknown" "yes"
      return 0
      ;;
    1|4)
      log_edge_decision "app_not_ready_for_guarded_runner" "$APP" "${PIN1:-?}" "$CARD" "unknown" "no"
      return 1
      ;;
    2)
      if [ "${PIN1:-}" = "0" ] || [ "${PIN1:-}" = "1" ]; then
        if remain_ok "$SIM"; then
          if [ "$ALLOW_PIN_VERIFY" = "1" ] && [ "$PIN_VERIFY_ATTEMPTED" = "0" ]; then
            log_edge_decision "pin_verify_window" "$APP" "$PIN1" "$CARD" "unknown" "yes" "remain=$REM"
            return 0
          fi
          log "PIN1=${PIN1}; VerifyPin unarmed or already attempted by this watcher"
        else
          log "PIN verification deferred: remaining attempts missing or <=1"
        fi
      fi
      log_edge_decision "app_pin_hold" "$APP" "${PIN1:-?}" "$CARD" "unknown" "no" "remain=${REM:-?}"
      log_soft_lock_status "app=$APP pin1=${PIN1:-?}"
      return 1
      ;;
    *)
      log_edge_decision "unrecognized_app" "$APP" "${PIN1:-?}" "$CARD" "unknown" "no"
      return 1
      ;;
  esac
}

# One-shot post-EDGE chase. A fresh SIM status still gates all writes.
if [ "${CHASE_ONCE:-0}" = "1" ]; then
  log "=== CHASE_ONCE (no tray-watch) ==="
  if ! sim_left_pin; then
    log "CHASE_ONCE no confirmed SIM state for chase"
    exit 1
  fi
  unlock_after_edge || true
  if chase_bearer; then
    log "GOAL bearer OK"
    exit 0
  fi
  log "CHASE_ONCE no bearer"
  exit 1
fi

# Persistent outer loop: re-arm watch batches until bearer or fatal OFFLINE.
BATCH=1
while :; do
  log "=== batch=$BATCH persist=$PERSIST rounds=$WATCH_ROUNDS (~$((WATCH_ROUNDS * 180))s) -- reseat anytime ==="

  # Immediate check: CardPower/tray may already have opened a VerifyPin window
  # before the first 180s tray-watch round.
  if sim_left_pin; then
    log "EDGE already present at start - unlock then chase (post_edge=yes)"
    unlock_after_edge || true
    if chase_bearer; then
      log "GOAL bearer OK"
      exit 0
    fi
    log "EDGE at start but no bearer yet - continue watch"
  fi

  ROUND=1
  while [ "$ROUND" -le "$WATCH_ROUNDS" ]; do
    ST=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
    UP=$(cut -d. -f1 /proc/uptime 2>/dev/null || echo '?')
    if [ "$ST" != ONLINE ]; then
      if [ "$PERSIST" = "1" ]; then
        log "WAIT STATE=$ST at batch=$BATCH round=$ROUND uptime_s=$UP - persist sleep 30s"
        sleep 30
        continue
      fi
      log "ABORT STATE=$ST at round=$ROUND uptime_s=$UP"
      exit 1
    fi
    log "--- batch=$BATCH round $ROUND/$WATCH_ROUNDS uptime_s=$UP ---"
    # Heartbeat for operators: confirm loop is alive without reading secrets.
    printf '%s\n' "alive batch=$BATCH round=$ROUND/$WATCH_ROUNDS uptime_s=$UP ts=$(date -Iseconds 2>/dev/null || date)" > /data/saaios/var/tray-bearer.alive
    ROUND_LOG=/tmp/tray-round-$BATCH-$ROUND.log
    : > "$ROUND_LOG"
    set +e
    "$SIT" tray-watch 2>&1 | tee -a "$OUT" | tee "$ROUND_LOG"
    TW=$?
    set -e
    log "tray-watch batch=$BATCH round=$ROUND exit=$TW"
    # Re-log every TRANSITION/ABSENT→PRESENT with post_edge decision.
    log_transitions_from_round "$ROUND_LOG"

    if sim_left_pin; then
      log "Fresh SIM status permits chase batch=$BATCH round=$ROUND"
      unlock_after_edge || true
      if chase_bearer; then
        log "GOAL bearer OK"
        exit 0
      fi
      log "No bearer yet - continue watch"
    else
      log "no EDGE this round batch=$BATCH round=$ROUND (post_edge=no) - soft-lock or idle"
    fi
    ROUND=$((ROUND + 1))
  done

  log "no fresh SIM status confirmed a usable app state in batch=$BATCH (${WATCH_ROUNDS} rounds); last state may be unknown"
  if [ "$PERSIST" != "1" ]; then
    log "GOAL incomplete (PERSIST=0)"
    exit 2
  fi
  log "PERSIST re-arm next batch (flock held) - observing published SIM state"
  BATCH=$((BATCH + 1))
  sleep 2
done
