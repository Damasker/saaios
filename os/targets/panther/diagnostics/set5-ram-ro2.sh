#!/bin/sh
# Keep PCIe warm with SIT, then ONE RO per phys for SET#5 CMP.
# No writes. Expect Thumb 90 f8 f6 0b 02 28 ...
ATU=ffffffe608f3a484
L1=ffffffe6094bd408
LK=ffffffe6094bd2a0
OUT=/data/saaios/var/set5-ram-ro2.txt
KO=/data/saaios/saaios_cp_poke.ko
SIG=90f8f60b0228
SIT=/data/saaios/bin/sit-sim-status

echo "start $(date -Iseconds)" > "$OUT"
echo "STATE:$(cat /sys/devices/platform/cpif/modem_state)" >> "$OUT"

warm() {
  i=0
  while [ $i -lt 4 ]; do
    timeout 3 $SIT query-sim-status >/dev/null 2>&1 || true
    w=$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width 2>/dev/null)
    echo "warm[$i]=$w" >> "$OUT"
    [ "$w" = "2" ] || [ "$w" = "1" ] && return 0
    i=$((i+1))
  done
  return 1
}

probe() {
  PHYS=$1
  USE_L1=$2
  warm || true
  rmmod saaios_cp_poke 2>/dev/null || true
  # one more SIT immediately before insmod
  timeout 3 $SIT query-sim-status >/data/saaios/var/set5-sim-snap2.txt 2>&1 || true
  echo "width_pre=$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width 2>/dev/null) PHYS=$PHYS L1=$USE_L1" >> "$OUT"
  if [ "$USE_L1" = "1" ]; then
    insmod "$KO" dry_run=0 read_only=1 \
      atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK \
      cp_phys=$PHYS ap_base=0x40200000 pcie_ch=0 \
      sig_hex=$SIG dump_n=16
  else
    insmod "$KO" dry_run=0 read_only=1 \
      atu_fn=0x$ATU link_fn=0x$LK \
      cp_phys=$PHYS ap_base=0x40200000 pcie_ch=0 \
      sig_hex=$SIG dump_n=16
  fi
  echo "PHYS=$PHYS RC=$?" >> "$OUT"
  dmesg | grep saaios_cp_poke | tail -16 >> "$OUT"
  rmmod saaios_cp_poke 2>/dev/null || true
}

# Prefer hyp PA + VA for SET#5 LDRB/CMP; include BTL base control
probe 0x814f49ac 0
probe 0x814f49ac 1
probe 0x414f49ac 0
probe 0x87200000 0
probe 0x814f49b0 0

echo "STATE_AFTER:$(cat /sys/devices/platform/cpif/modem_state)" >> "$OUT"
grep -E 'app0_state|pin1_state' /data/saaios/var/set5-sim-snap2.txt >> "$OUT" || true
echo "rmnet0_rx=$(cat /sys/class/net/rmnet0/statistics/rx_packets 2>/dev/null)" >> "$OUT"
echo DONE >> "$OUT"
cat "$OUT"
