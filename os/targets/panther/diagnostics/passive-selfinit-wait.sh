#!/bin/sh
# Minimal soft ONLINE + passive USIM self-init watch.
# No EngMode / CardPower / OemSim / 0x0704 / VerifyPin / probe-no0200.
# Polls only 0x0200 + data-reg + rmnet for ~4 minutes.
set -eu
# Return an observation, never a claim about unobserved CP memory.
classify_sample() {
  [ "$1" = 0 ] && [ "$2" = ONLINE ] && [ "$3" = 1 ] || { echo inconclusive; return; }
  case "$4:$5" in
    5:*) echo ready ;;
    2:1) echo pin_required ;;
    2:2|2:3) echo pin_state_stalled ;;
    *) echo inconclusive ;;
  esac
}
if [ "${1:-}" = --self-test ]; then
  [ "$(classify_sample 1 ONLINE 1 2 2)" = inconclusive ]
  [ "$(classify_sample 0 OFFLINE 1 2 2)" = inconclusive ]
  [ "$(classify_sample 0 ONLINE 0 2 2)" = inconclusive ]
  [ "$(classify_sample 0 ONLINE 1 '' '')" = inconclusive ]
  [ "$(classify_sample 0 ONLINE 1 1 2)" = inconclusive ]
  [ "$(classify_sample 0 ONLINE 1 4 2)" = inconclusive ]
  [ "$(classify_sample 0 ONLINE 1 2 1)" = pin_required ]
  [ "$(classify_sample 0 ONLINE 1 2 2)" = pin_state_stalled ]
  [ "$(classify_sample 0 ONLINE 1 2 3)" = pin_state_stalled ]
  [ "$(classify_sample 0 ONLINE 1 5 2)" = ready ]
  echo 'PASS: sample classification (no device I/O)'
  exit 0
fi
mkdir -p /data/saaios/var
LOG=/data/saaios/var/passive-selfinit-wait.log
: > "$LOG"
log() { printf '%s\n' "$*" | tee -a "$LOG"; }

present_infer_from_app() {
  # STATUS Present→SET: 0→PIN(2), 1→PUK(3), 2→READY(5), 3→PERSO(4)
  case "$1" in
    2) echo 0 ;;
    3) echo 1 ;;
    5) echo 2 ;;
    4) echo 3 ;;
    *) echo na ;;
  esac
}

snap() {
  tag="$1"
  up=$(cut -d' ' -f1 /proc/uptime)
  st=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo NONE)
  log "SNAP $tag UP=$up STATE=$st"
  query_rc=0
  out=$(/data/saaios/bin/sit-sim-status query-sim-status 2>&1) || query_rc=$?
  printf '%s\n' "$out" | tee -a "$LOG"
  app=$(printf '%s\n' "$out" | sed -n 's/.*app0_state_raw=\([0-9]*\).*/\1/p' | head -1)
  pin1=$(printf '%s\n' "$out" | sed -n 's/.*pin1_state_raw=\([0-9]*\).*/\1/p' | head -1)
  remain=$(printf '%s\n' "$out" | sed -n 's/.*pin1_remain_raw=\([0-9]*\).*/\1/p' | head -1)
  card=$(printf '%s\n' "$out" | sed -n 's/.*card_state_raw=\([0-9]*\).*/\1/p' | head -1)
  pi=unknown
  observation=$(classify_sample "$query_rc" "$st" "${card:-}" "${app:-}" "${pin1:-}")
  log "PARSE card=${card:--} app=${app:--} pin1=${pin1:--} remain=${remain:--} present_infer=$pi query_rc=$query_rc observation=$observation"
  reg=$(/data/saaios/bin/sit-sim-status query-data-registration 2>&1 || true)
  printf '%s\n' "$reg" | tee -a "$LOG"
  for rif in 0 1 2 3 4 5; do
    rx=$(cat /sys/class/net/rmnet$rif/statistics/rx_bytes 2>/dev/null || echo -)
    tx=$(cat /sys/class/net/rmnet$rif/statistics/tx_bytes 2>/dev/null || echo -)
    log " rmnet$rif rx=$rx tx=$tx"
  done
  ip -4 -o addr show 2>/dev/null | grep rmnet | tee -a "$LOG" || log " no_rmnet_ipv4"
  # Export last parse for caller logic via files
  printf '%s' "${app:-}" > /data/saaios/var/passive-last-app
  printf '%s' "${pin1:-}" > /data/saaios/var/passive-last-pin1
  printf '%s' "$pi" > /data/saaios/var/passive-last-present
}

log "BEGIN passive-selfinit-wait UP=$(cut -d' ' -f1 /proc/uptime)"
# Never unlink a flock file: an existing owner would retain the old inode.
if ps | grep -E '[t]ray-bearer-chase|[s]it-sim-status|[p]ost-init-chase' >/dev/null; then
  log 'REFUSE: another diagnostic client is active'
  exit 90
fi
ps | grep -E '[c]bd|[r]ild' && { log "REFUSE: cbd/rild present"; exit 90; } || log "NO_cbd_rild"

# CPIF must be loaded before modem_state exists (post-sysrq = NONE).
insmod /lib/modules/shm_ipc.ko 2>/dev/null || true
insmod /lib/modules/cpif_page.ko 2>/dev/null || true
insmod /lib/modules/cpif.ko 2>/dev/null || true
insmod /lib/modules/cp_thermal_zone.ko 2>/dev/null || true

STATE=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo NONE)
log "STATE0=$STATE"
if [ "$STATE" != ONLINE ]; then
  [ "$STATE" = OFFLINE ] || { log "need OFFLINE or ONLINE got=$STATE"; exit 1; }
  log "bring-up: probe-handover-clean only (no probe-no0200 / EngMode / CardPower / VerifyPin)"
  log "STATE_pre=$STATE"
  ln -sf /data/saaios/bin/saaios-probe-b-modem.bin /tmp/saaios-probe-b-modem.bin
  cp -a /data/saaios/bin/probe-handover-clean /tmp/probe-handover
  cp -a /data/saaios/bin/saaios-verify-nv-copies.sh /tmp/saaios-verify-nv-copies.sh
  chmod +x /tmp/probe-handover /tmp/saaios-verify-nv-copies.sh
  mkdir -p /mnt/vendor/persist /dev/block /data/saaios/var
  if [ ! -e /dev/block/sda1 ]; then
    set -- $(cat /sys/block/sda/sda1/dev | tr : ' ')
    mknod /dev/block/sda1 b "$1" "$2"
  fi
  for n in umts_boot0 umts_ipc0 umts_rfs0; do
    if [ ! -e /dev/$n ]; then
      set -- $(cat /sys/class/cpif/$n/dev | tr : ' ')
      mknod /dev/$n c "$1" "$2"
    fi
  done
  if grep -q ' /mnt/vendor/persist ' /proc/mounts; then
    log 'REFUSE: persist is already mounted by another owner'
    exit 90
  fi
  mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 /mnt/vendor/persist
  trap 'umount /mnt/vendor/persist' EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM
  PRC=0
  /tmp/probe-handover boot-b-with-verified-nv-handover /mnt/vendor/persist/modem/cpsha >> "$LOG" 2>&1 || PRC=$?
  umount /mnt/vendor/persist
  trap - EXIT INT TERM
  log "probe_rc=$PRC STATE=$(cat /sys/devices/platform/cpif/modem_state)"
  [ "$PRC" = 0 ] || exit "$PRC"
  [ "$(cat /sys/devices/platform/cpif/modem_state)" = ONLINE ] || { log "not ONLINE"; exit 2; }
  if grep -E 'HOLD RESULT|SIM length=|query-sim|0x0200' "$LOG" >/dev/null 2>&1; then
    log "CONTAMINATED: handover sent SIM"
    exit 4
  else
    log "ONLINE clean (no early 0x0200 in handover log)"
  fi
else
  log "already ONLINE — passive window only (no re-boot)"
fi

# First poll immediately, then every 20s for ~4 minutes (13 samples).
READY_SEEN=0
VALID_SAMPLES=0
PIN_REQUIRED_SAMPLES=0
i=0
while [ "$i" -lt 13 ]; do
  snap "t$i"
  case "$observation" in
    ready)
      READY_SEEN=1
      log "SELFINIT_OR_READY app=$app — stop passive; chase may follow"
      break
      ;;
    pin_required) PIN_REQUIRED_SAMPLES=$((PIN_REQUIRED_SAMPLES + 1)); VALID_SAMPLES=$((VALID_SAMPLES + 1)) ;;
    pin_state_stalled) VALID_SAMPLES=$((VALID_SAMPLES + 1)) ;;
  esac
  i=$((i + 1))
  [ "$i" -lt 13 ] && sleep 20
done

log "SUMMARY READY_SEEN=$READY_SEEN final_app=$(cat /data/saaios/var/passive-last-app) final_pin1=$(cat /data/saaios/var/passive-last-pin1) present_infer=$(cat /data/saaios/var/passive-last-present)"
if [ "$READY_SEEN" = 1 ]; then
  log "VERDICT: app left PIN — bearer chase warranted"
  exit 0
fi
if [ "$VALID_SAMPLES" != 13 ]; then
  log "VERDICT: inconclusive valid_samples=$VALID_SAMPLES expected=13"
  exit 4
fi
if [ "$PIN_REQUIRED_SAMPLES" != 0 ]; then
  log "VERDICT: CP reports unverified PIN in $PIN_REQUIRED_SAMPLES samples; card-level PIN setting and readiness after verification NOT tested"
  exit 5
fi
log "VERDICT: READY not observed in this bounded window; cause and live Present remain unknown"
exit 3
