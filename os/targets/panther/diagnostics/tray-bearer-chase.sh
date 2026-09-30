#!/bin/sh
# After physical SIM tray reseat: watch soft-lock edge, then proven
# CardPower-VerifyPin A (AID) if needed, then Radio/LTE/reg/rmnet.
# Never logs PIN digits or AID. Stop if remain<=1. No POWER_OFF / crash.
#
# Handles both soft-lock shapes:
#   app=PIN + pin1=DISABLED(3)
#   app=PIN + pin1=ENABLED_VERIFIED(2)  (Pin1Verified OK; still no START_NETWORK)
#
# Post-EDGE (RE): READY needs Present/+0xBF6==2 (FN_A CDMA only; EU No-CDMA
# RatMap => remote cannot force READY). Once app?{1,4,5}:
#   VerifyPin(if pin1 0|1) -> gate START_NETWORK -> Radio -> LTE_ONLY ->
#   NetworkSelectionAuto -> AllowData -> GetPsService -> data-reg ->
#   GetDataCallList -> SetupDataCall(if APN) -> rmnet IPv4/rx+tx
#
# WATCH_ROUNDS: each tray-watch is ~180s; default 12 (~36 min).
# PERSIST=1: when rounds expire without bearer, re-arm another batch
#   (same flock); reseat hours later still triggers chase. Exit only on
#   BEARER_OK or fatal OFFLINE/ABORT.
# VERIFY_TOOL: optional RFS-aware CardPower+VerifyPin A binary
#   (default: /tmp then /data/saaios/bin card-then-verify-a).
# APN_FILE: /data/saaios/etc/apn (operator-supplied host; never invent).
set -eu
SIT=${SIT:-/data/saaios/bin/sit-sim-status}
BIN=${BIN:-/data/saaios/bin}
OUT=${OUT:-/data/saaios/var/tray-bearer.log}
WATCH_ROUNDS=${WATCH_ROUNDS:-12}
BEARER_POLLS=${BEARER_POLLS:-60}
PERSIST=${PERSIST:-1}
APN_FILE=${APN_FILE:-/data/saaios/etc/apn}
mkdir -p "$(dirname "$OUT")" /run /data/saaios/etc
{
  printf '=== tray-bearer-chase start %s rounds=%s persist=%s ===\n' "$(date -Iseconds 2>/dev/null || date)" "$WATCH_ROUNDS" "$PERSIST"
} >> "$OUT"
log() { printf '%s\n' "$*" | tee -a "$OUT"; }

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
if [ -z "$VERIFY_TOOL" ]; then
  VERIFY_TOOL=$(resolve_tool card-then-verify-a || true)
fi
ALLOW_DATA=$(resolve_tool allow-data-once || true)
GET_PS=$(resolve_tool get-ps-service || true)
RADIO_ON=$(resolve_tool radio-cycle-no-verify || true)
GET_DCL=$(resolve_tool get-data-call-list || true)
SETUP_DC=$(resolve_tool setup-data-call || true)

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

log "TOOLS verify=${VERIFY_TOOL:-none} allow=${ALLOW_DATA:-none} getps=${GET_PS:-none} radio=${RADIO_ON:-none} getdcl=${GET_DCL:-none} setupdc=${SETUP_DC:-none} persist=$PERSIST"
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

# Mirror sit-sim-status present_infer (0x0200 has no +0xBF6).
# Validated 2026-09-30: byte17=+0xBF4 (GET_APP), not Present/+0xBF6.
# Maps last STATUS→SET_APP decision: Present 1→PUK(#3), 2→READY(#5),
# 3→PERSO(#4), else (incl. 0)→PIN→notin_1_2_3. HotSwap live: still notin.
present_infer_from_app() {
  case "${1:-}" in
    5) printf '%s\n' was_2 ;;
    3) printf '%s\n' was_1 ;;
    4) printf '%s\n' was_3 ;;
    2) printf '%s\n' notin_1_2_3 ;;
    1) printf '%s\n' 'n/a(DETECTED)' ;;
    0) printf '%s\n' 'n/a(UNKNOWN)' ;;
    *) printf '%s\n' 'n/a' ;;
  esac
}

# Decisive falsifier line on EDGE/ABSENT/PRESENT (no secrets).
# post_edge=yes means unlock+chase will run; no means soft-lock hold.
log_edge_decision() {
  # usage: log_edge_decision <tag> <app> <pin1> <card> <present_infer> <post_edge_yes_no> [extra]
  log "EDGE_DECISION tag=$1 app=${2:-?} pin1=${3:-?} card=${4:-?} present_infer=${5:-?} post_edge=$6 ${7:-}"
}

remain_ok() {
  R=$(sim_field pin1_remain_raw "$1")
  [ -n "$R" ] || return 0
  [ "$R" -gt 1 ] 2>/dev/null
}

# START_NETWORK / camp gate (CP GET_APP CMP #1/#4/#5).
app_start_network_ok() {
  case "${1:-}" in
    1|4|5) return 0 ;;
    *) return 1 ;;
  esac
}

# After EDGE: if pin1==1, CardPower+VerifyPin A with AID (proven remote path).
# pin1=2 already ENABLED_VERIFIED - skip CardPower (would reset to 1).
# Logs only app/pin1/remain/error - never PIN/AID bytes.
unlock_after_edge() {
  log "=== phase1b Pin1Verified attempt (no secrets) ==="
  rm -f /run/saaios-sit-status.lock
  set +e
  SIM=$("$SIT" query-sim-status 2>/dev/null | tr '\n' ' ')
  set -e
  APP=$(sim_field app0_state_raw "$SIM")
  PIN1=$(sim_field pin1_state_raw "$SIM")
  REM=$(sim_field pin1_remain_raw "$SIM")
  log "pre_unlock app_raw=${APP:-?} pin1=${PIN1:-?} remain=${REM:-?}"

  if ! remain_ok "$SIM"; then
    log "STOP remain<=1 - VerifyPin not sent"
    return 1
  fi

  # Already READY / DETECTED / PERSO - skip verify
  if app_start_network_ok "${APP:-}"; then
    log "app already ${APP} (START_NETWORK set) - skip VerifyPin"
    return 0
  fi

  # pin1=2 (ENABLED_VERIFIED) - Pin1Verified already; skip CardPower (would reset)
  if [ "${PIN1:-}" = "2" ]; then
    log "pin1=2 already ENABLED_VERIFIED - skip CardPower/VerifyPin"
    return 0
  fi

  if [ -n "${VERIFY_TOOL:-}" ] && [ -x "$VERIFY_TOOL" ]; then
    log "running VERIFY_TOOL (CardPower then VerifyPin A+AID if pin1=1)"
    set +e
    "$VERIFY_TOOL" 2>&1 | tee -a "$OUT" | grep -E '^(pre|after|SIM |card |radio |VerifyPin |RESULT|CONFIRMED|STOP|pin1=|B skipped|TOOL)' || true
    set -e
  else
    log "VERIFY_TOOL missing - Radio chase only"
  fi

  set +e
  SIM2=$("$SIT" query-sim-status 2>/dev/null | tr '\n' ' ')
  set -e
  log "post_unlock app_raw=$(sim_field app0_state_raw "$SIM2") pin1=$(sim_field pin1_state_raw "$SIM2") remain=$(sim_field pin1_remain_raw "$SIM2")"
  return 0
}

chase_bearer() {
  log "=== phase2 bearer chase (post-EDGE pipeline) ==="
  rm -f /run/saaios-sit-status.lock

  # Gate: START_NETWORK only allows app in {1,4,5}.
  set +e
  SIM0=$("$SIT" query-sim-status 2>/dev/null | tr '\n' ' ')
  set -e
  APP0=$(sim_field app0_state_raw "$SIM0")
  PIN10=$(sim_field pin1_state_raw "$SIM0")
  log "chase_gate app_raw=${APP0:-?} pin1=${PIN10:-?} (START_NETWORK needs 1|4|5)"

  START_OK=0
  if app_start_network_ok "${APP0:-}"; then
    START_OK=1
    log "chase_gate START_NETWORK_ALLOWED=yes - running LTE/AllowData/GetPs/reg/rmnet"
  else
    log "chase_gate START_NETWORK_ALLOWED=no (app=${APP0:-?}) - Radio/LTE/AllowData still run; PS may stay idle until reseat READY"
  fi

  set +e
  "$SIT" query-radio-state 2>&1 | tee -a "$OUT"
  # Prefer ON-only if available; avoid full radio-power-cycle VerifyPin empty
  if [ -n "${RADIO_ON:-}" ] && [ -x "$RADIO_ON" ]; then
    "$RADIO_ON" 2>&1 | tee -a "$OUT" || true
  elif "$SIT" 2>&1 | grep -q radio-power-cycle; then
    "$SIT" radio-power-cycle 2>&1 | tee -a "$OUT"
  fi

  # LTE_ONLY (preferred=11) then auto selection (0x0704) - proven signed path.
  "$SIT" set-preferred-lte 2>&1 | tee -a "$OUT"
  if "$SIT" 2>&1 | grep -q query-preferred-network; then
    "$SIT" query-preferred-network 2>&1 | tee -a "$OUT" || true
  fi
  "$SIT" set-network-selection-auto 2>&1 | tee -a "$OUT"

  if [ -n "${ALLOW_DATA:-}" ] && [ -x "$ALLOW_DATA" ]; then
    log "AllowData 0x0710 allow=1 via $ALLOW_DATA"
    "$ALLOW_DATA" 2>&1 | tee -a "$OUT" || true
  else
    log "AllowData tool missing - skip 0x0710"
  fi

  # Proven empty GET after Radio/LTE; helpful once app?{1,4,5}.
  if [ -n "${GET_PS:-}" ] && [ -x "$GET_PS" ]; then
    log "GetPsService 0x0711 via $GET_PS"
    "$GET_PS" 2>&1 | tee -a "$OUT" || true
  fi
  if [ -n "${GET_DCL:-}" ] && [ -x "$GET_DCL" ]; then
    log "GetDataCallList 0x0602 via $GET_DCL"
    "$GET_DCL" 2>&1 | tee -a "$OUT" || true
  fi
  # SetupDataCall 0x0600 len246 only when APN file/tool present (no invent).
  if [ "$START_OK" -eq 1 ] && [ -n "${SETUP_DC:-}" ] && [ -x "$SETUP_DC" ]; then
    if [ -r "$APN_FILE" ]; then
      log "SetupDataCall 0x0600 via $SETUP_DC (APN from $APN_FILE; string not logged)"
      set +e
      "$SETUP_DC" --apn-file "$APN_FILE" 2>&1 | tee -a "$OUT" | grep -E '^(SetupDataCall|apn_hint|requires|lock|open|write|ipc )' || true
      set -e
    else
      log "SetupDataCall deferred_no_apn (write carrier APN to $APN_FILE)"
    fi
  elif [ "$START_OK" -eq 1 ]; then
    log "SetupDataCall tool missing - skip 0x0600"
  fi
  set -e

  i=0
  while [ "$i" -lt "$BEARER_POLLS" ]; do
    set +e
    "$SIT" query-data-registration 2>&1 | tee -a "$OUT"
    REG=$("$SIT" query-data-registration 2>/dev/null | tr '\n' ' ')
    set -e
    # Mid-poll: if reg looks active, re-hit GetPsService (signed AP=3/CP=7 path).
    if echo "$REG" | grep -qiE 'registered|reg_state_raw=[1-5]|registration_raw=[1-5]'; then
      if [ -n "${GET_PS:-}" ] && [ -x "$GET_PS" ]; then
        set +e
        "$GET_PS" 2>&1 | tee -a "$OUT" || true
        set -e
      fi
      # Re-assert LTE_ONLY + AllowData once camp looks alive.
      if [ "$START_OK" -eq 1 ]; then
        set +e
        "$SIT" set-preferred-lte 2>&1 | tee -a "$OUT" || true
        if [ -n "${ALLOW_DATA:-}" ] && [ -x "$ALLOW_DATA" ]; then
          "$ALLOW_DATA" 2>&1 | tee -a "$OUT" || true
        fi
        if [ -n "${SETUP_DC:-}" ] && [ -x "$SETUP_DC" ] && [ -r "$APN_FILE" ]; then
          "$SETUP_DC" --apn-file "$APN_FILE" 2>&1 | tee -a "$OUT" | grep -E '^(SetupDataCall|apn_hint|requires|lock|open|write|ipc )' || true
        fi
        set -e
      fi
    fi
    echo "$REG" | grep -qiE 'registered|reg_state_raw=[1-5]' && break
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

edge_ready() {
  F=$1
  # ASCII-only patterns: UTF-8 arrows in this script can corrupt over USB text
  # transfer and fail to match tray-watch's ABSENT-PRESENT line.
  grep -qE 'saw_ready=1|saw_detected=1|app=1\(|app=5\(|app=4\(|ABSENT.PRESENT|tray-watch: pin1 changed' "$F" && return 0
  return 1
}

# Re-emit tray-watch TRANSITION / ABSENT→PRESENT with post_edge decision.
# Physical reseat becomes a decisive falsifier in tray-bearer.log alone.
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
        PI=$(echo "$LINE" | tr ' ' '\n' | grep -E '^present_infer=' | head -1 | cut -d= -f2)
        [ -n "$PI" ] || PI=$(present_infer_from_app "${APP:-}")
        POST=no
        case "${APP:-}" in
          0|1|4|5) POST=yes ;;
          2)
            case "${PIN1:-}" in 0|1) POST=yes ;; esac
            ;;
        esac
        [ "${CARD:-}" = "0" ] && POST=yes
        log_edge_decision "transition" "${APP:-?}" "${PIN1:-?}" "${CARD:-?}" "${PI:-?}" "$POST" "src=tray-watch"
        ;;
      *ABSENT*PRESENT*|*'ABSENT→PRESENT'*|*'ABSENT-PRESENT'*)
        log_edge_decision "absent_present" "?" "?" "?" "?" "yes" "src=tray-watch"
        ;;
    esac
  done < "$F"
}

sim_left_pin() {
  # Empty/failed query must NOT count as EDGE (false chase on soft-lock).
  # BusyBox ash grep needs -E for alternation; BRE \| does not match and
  # previously fell through to return 0 -> false EDGE while still PIN+pin1=2.
  set +e
  SIM=$("$SIT" query-sim-status 2>/dev/null | tr '\n' ' ')
  RC=$?
  set -e
  # Log non-secret fields only
  APP=$(sim_field app0_state_raw "$SIM")
  PIN1=$(sim_field pin1_state_raw "$SIM")
  REM=$(sim_field pin1_remain_raw "$SIM")
  CARD=$(sim_field card_state_raw "$SIM")
  PI=$(present_infer_from_app "${APP:-}")
  log "SIM snapshot app=${APP:-?} pin1=${PIN1:-?} remain=${REM:-?} card=${CARD:-?} present_infer=$PI"
  [ "$RC" -eq 0 ] || return 1
  [ -n "$SIM" ] || return 1
  echo "$SIM" | grep -qE 'app0_state=|app0_state_raw=|card_state=' || return 1
  # CardPower / tray may leave app=PIN but pin1 NOT_VERIFIED(1) or UNKNOWN(0):
  # that is a real VerifyPin window (EDGE), not the pin1=2 soft-lock chicken-egg.
  if [ "${APP:-}" = "2" ]; then
    case "${PIN1:-}" in
      0|1)
        log_edge_decision "pin_verify_window" "$APP" "$PIN1" "${CARD:-?}" "$PI" "yes" "remain=${REM:-?}"
        log "PIN but pin1=${PIN1} (NOT_VERIFIED/UNKNOWN) - EDGE for VerifyPin"
        return 0
        ;;
    esac
    log_edge_decision "soft_lock_hold" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "no" "remain=${REM:-?}"
    log "still PIN (app_raw=2 pin1=${PIN1:-?}) - not EDGE"
    return 1
  fi
  if echo "$SIM" | grep -qE 'app0_state=READY|app0_state_raw=5'; then
    log_edge_decision "ready" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'app0_state=DETECTED|app0_state_raw=1'; then
    log_edge_decision "detected" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'app0_state=SUBSCRIPTION_PERSO|app0_state_raw=4'; then
    log_edge_decision "perso" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'app0_state=UNKNOWN|app0_state_raw=0'; then
    log_edge_decision "unknown_app" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'card_state=ABSENT|card_state_raw=0'; then
    log_edge_decision "card_absent" "${APP:-?}" "${PIN1:-?}" "${CARD:-0}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'pin1_state_raw=1|pin1_state_raw=0'; then
    log_edge_decision "pin1_unverified" "${APP:-?}" "${PIN1:-?}" "${CARD:-?}" "$PI" "yes"
    return 0
  fi
  if echo "$SIM" | grep -qE 'app0_state=PIN|app0_state_raw=2'; then
    log_edge_decision "soft_lock_hold" "$APP" "${PIN1:-?}" "${CARD:-?}" "$PI" "no"
    return 1
  fi
  # Unknown shape: do not chase.
  log_edge_decision "unrecognized" "${APP:-?}" "${PIN1:-?}" "${CARD:-?}" "$PI" "no"
  log "unrecognized SIM snapshot - not EDGE"
  return 1
}

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

    if edge_ready "$ROUND_LOG" || sim_left_pin; then
      log "EDGE observed batch=$BATCH round=$ROUND - unlock then chase (post_edge=yes)"
      unlock_after_edge || true
      if chase_bearer; then
        log "GOAL bearer OK"
        exit 0
      fi
      log "EDGE but no IPv4 yet - continue watch"
    else
      log "no EDGE this round batch=$BATCH round=$ROUND (post_edge=no) - soft-lock or idle"
    fi
    ROUND=$((ROUND + 1))
  done

  log "no DETECTED/READY/ABSENT edge in batch=$BATCH (${WATCH_ROUNDS} rounds) - soft-lock holds"
  if [ "$PERSIST" != "1" ]; then
    log "GOAL incomplete (PERSIST=0)"
    exit 2
  fi
  log "PERSIST re-arm next batch (flock held) - reseat still watched"
  BATCH=$((BATCH + 1))
  sleep 2
done
