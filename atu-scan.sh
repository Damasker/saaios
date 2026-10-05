#!/bin/sh
# RO ATU OB2 scan for MAIN SET#2 (02 20) / TOC. No writes.
# busybox ash compatible.
set -eu
OUT=/data/saaios/var/atu-scan.log
: > "$OUT"
log() { printf '%s\n' "$*" | tee -a "$OUT"; }

STATE=$(cat /sys/devices/platform/cpif/modem_state)
log "STATE=$STATE"
[ "$STATE" = ONLINE ] || { log "need ONLINE"; exit 1; }

rm -f /run/saaios-sit-status.lock
/data/saaios/bin/sit-sim-status query-radio-state >/dev/null 2>&1 || true

ATU=$(grep exynos_pcie_rc_set_outbound_atu /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
L1=$(grep s51xx_pcie_l1ss_ctrl /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
LK=$(grep exynos_pcie_rc_chk_link_status /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
log "ATU=$ATU L1=$L1 LK=$LK"
log "link=$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width 2>/dev/null || echo ?)"

KO=/data/saaios/saaios_cp_poke.ko
# SET#2 VA 0x414f47f4 low20 = 0xf47f4; bytes 02 20 96 f0 64 f1 6e 78
SIG_SET2=022096f064f16e78
SIG_TOC=544f4300
SIG_MAIN=88f09fe5
HITS=0

probe_one() {
  PHYS=$1
  TAG=$2
  SIG=$3
  rmmod saaios_cp_poke 2>/dev/null || true
  dmesg -C 2>/dev/null || true
  if ! insmod "$KO" dry_run=0 read_only=1 \
      atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK \
      cp_phys=$PHYS ap_base=0x40200000 pcie_ch=0 \
      dump_n=16 sig_hex=$SIG 2>>"$OUT"; then
    log "FAIL $TAG phys=$PHYS"
    ST=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
    log "after_fail STATE=$ST"
    rmmod saaios_cp_poke 2>/dev/null || true
    [ "$ST" = ONLINE ] || return 2
    return 1
  fi
  DUMP=$(dmesg | grep 'saaios_cp_poke: DUMP' | tail -1)
  SIGL=$(dmesg | grep 'saaios_cp_poke: SIG\|WIN all-ff\|ATU failed\|ATU try' | tail -4 | tr '\n' ';')
  log "$TAG $PHYS | $DUMP | $SIGL"
  rmmod saaios_cp_poke 2>/dev/null || true
  if echo "$SIGL" | grep -q 'SIG HIT'; then
    HITS=$((HITS + 1))
    log "FOUND_HIT phys=$PHYS tag=$TAG"
    return 10
  fi
  return 0
}

log "SIG_SET2=$SIG_SET2"

# Confirm priors
probe_one 0x814f47f4 prior-hyp "$SIG_SET2" || true
probe_one 0x87200000 prior-btl "$SIG_TOC" || true
probe_one 0x414f47f4 prior-va "$SIG_SET2" || true
probe_one 0x40010000 prior-mainva "$SIG_MAIN" || true
probe_one 0x80010000 prior-mainpa "$SIG_MAIN" || true

# Coarse 16MiB bands — SET#2 offset in window
# shell loop with hex via printf
scan_set2_band() {
  NAME=$1
  START=$2
  END=$3
  STEP=$4
  log "BAND $NAME start=$START end=$END step=$STEP"
  P=$START
  while true; do
    # stop when P >= END (unsigned compare via busybox)
    PL=$(printf '%u' "$P")
    EL=$(printf '%u' "$END")
    [ "$PL" -lt "$EL" ] || break
    ST=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
    if [ "$ST" != ONLINE ]; then
      log "ABORT STATE=$ST at $P"
      return 2
    fi
    PHYS=$(printf '0x%x' $((P + 0xf47f4)))
    probe_one "$PHYS" "$NAME" "$SIG_SET2" && RC=0 || RC=$?
    [ "$RC" = 10 ] && return 10
    [ "$RC" = 2 ] && return 2
    # also TOC/base if non-trivial
    probe_one "$(printf '0x%x' $P)" "${NAME}b" "$SIG_TOC" && RC=0 || RC=$?
    [ "$RC" = 10 ] && return 10
    [ "$RC" = 2 ] && return 2
    P=$(printf '0x%x' $((P + STEP)))
  done
  return 0
}

RC=0
scan_set2_band b800 0x80000000 0x90000000 0x1000000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

scan_set2_band b400 0x40000000 0x50000000 0x1000000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

scan_set2_band b000 0x00000000 0x10000000 0x1000000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

scan_set2_band ba00 0xa0000000 0xb0000000 0x1000000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

scan_set2_band bc00 0xc0000000 0xd0000000 0x1000000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

# Fine 1MiB: BTL ±16M and hyp SET2 ±8M
scan_set2_band fBTL 0x86000000 0x89000000 0x100000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

scan_set2_band fHYP 0x80c00000 0x81c00000 0x100000 || RC=$?
[ "$RC" = 10 ] && { log "DONE HIT hits=$HITS"; exit 0; }
[ "$RC" = 2 ] && { log "NEED_REBOOT"; exit 2; }

log "SCAN_NEGATIVE no SIG HIT hits=$HITS"
log "FINAL STATE=$(cat /sys/devices/platform/cpif/modem_state)"
exit 1
