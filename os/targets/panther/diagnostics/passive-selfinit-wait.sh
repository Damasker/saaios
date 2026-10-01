#!/bin/sh
# Minimal soft ONLINE + passive USIM self-init watch.
# No EngMode / CardPower / OemSim / 0x0704 / VerifyPin / probe-no0200.
# Polls only 0x0200 + data-reg + rmnet for ~4 minutes.
set -eu
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
  out=$(/data/saaios/bin/sit-sim-status query-sim-status 2>&1 || true)
  printf '%s\n' "$out" | tee -a "$LOG"
  app=$(printf '%s\n' "$out" | sed -n 's/.*app0_state_raw=\([0-9]*\).*/\1/p' | head -1)
  pin1=$(printf '%s\n' "$out" | sed -n 's/.*pin1_state_raw=\([0-9]*\).*/\1/p' | head -1)
  remain=$(printf '%s\n' "$out" | sed -n 's/.*pin1_remain_raw=\([0-9]*\).*/\1/p' | head -1)
  card=$(printf '%s\n' "$out" | sed -n 's/.*card_state_raw=\([0-9]*\).*/\1/p' | head -1)
  pi=$(present_infer_from_app "${app:-x}")
  log "PARSE card=${card:--} app=${app:--} pin1=${pin1:--} remain=${remain:--} present_infer=$pi"
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
pkill -f tray-bearer-chase 2>/dev/null || true
pkill -f 'sit-sim-status' 2>/dev/null || true
rm -f /run/saaios-sit-status.lock
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
  umount /mnt/vendor/persist 2>/dev/null || true
  mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 /mnt/vendor/persist
  /tmp/probe-handover boot-b-with-verified-nv-handover /mnt/vendor/persist/modem/cpsha >> "$LOG" 2>&1
  PRC=$?
  umount /mnt/vendor/persist || true
  log "probe_rc=$PRC STATE=$(cat /sys/devices/platform/cpif/modem_state)"
  [ "$PRC" = 0 ] || exit "$PRC"
  [ "$(cat /sys/devices/platform/cpif/modem_state)" = ONLINE ] || { log "not ONLINE"; exit 2; }
  if grep -E 'HOLD RESULT|SIM length=|query-sim|0x0200' "$LOG" >/dev/null 2>&1; then
    log "CONTAMINATED: handover sent SIM"
  else
    log "ONLINE clean (no early 0x0200 in handover log)"
  fi
else
  log "already ONLINE — passive window only (no re-boot)"
fi

# First poll immediately, then every 20s for ~4 minutes (13 samples).
READY_SEEN=0
i=0
while [ "$i" -lt 13 ]; do
  snap "t$i"
  app=$(cat /data/saaios/var/passive-last-app 2>/dev/null || echo)
  case "$app" in
    1|4|5)
      READY_SEEN=1
      log "SELFINIT_OR_READY app=$app — stop passive; chase may follow"
      break
      ;;
  esac
  i=$((i + 1))
  [ "$i" -lt 13 ] && sleep 20
done

log "SUMMARY READY_SEEN=$READY_SEEN final_app=$(cat /data/saaios/var/passive-last-app) final_pin1=$(cat /data/saaios/var/passive-last-pin1) present_infer=$(cat /data/saaios/var/passive-last-present)"
if [ "$READY_SEEN" = 1 ]; then
  log "VERDICT: app left PIN — bearer chase warranted"
  exit 0
fi
log "VERDICT: stayed PIN entire passive window — CP USIM self-init hypothesis FALSIFIED for this soft path"
exit 3
