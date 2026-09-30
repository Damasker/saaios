#!/bin/sh
# RO-only SET#5 Present CMP site via modem ATU. No write.
# Expect Thumb @file 0x14fb5bc: 90 f8 f6 0b 02 28 36 d1 (LDRB+0xBF6 CMP#2 BNE)
set -e
ATU=ffffffe608f3a484
L1=ffffffe6094bd408
LK=ffffffe6094bd2a0
OUT=/data/saaios/var/set5-ram-ro.txt
KO=/data/saaios/saaios_cp_poke.ko
SIG=90f8f60b0228

echo "start $(date -Iseconds)" > "$OUT"
echo "STATE:$(cat /sys/devices/platform/cpif/modem_state)" >> "$OUT"
echo "ATU=$ATU L1=$L1 LK=$LK" >> "$OUT"
echo "width=$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width 2>/dev/null)" >> "$OUT"

# Warm PCIe lightly without fighting tray-watch long (best-effort)
if [ -x /data/saaios/bin/sit-sim-status ]; then
  timeout 8 /data/saaios/bin/sit-sim-status query-sim-status \
    >/data/saaios/var/set5-sim-snap.txt 2>&1 || true
  echo "SIM_RC=$?" >> "$OUT"
  grep -E 'app_state|pin1|card|PRESENT|PIN|READY|error' \
    /data/saaios/var/set5-sim-snap.txt 2>/dev/null | head -20 >> "$OUT" || true
fi
echo "width_after_sit=$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width 2>/dev/null)" >> "$OUT"

for PHYS in 0x414f49ac 0x414f49b0 0x814f49ac 0x814f49b0 0x414f47f4 0x814f47f4; do
  rmmod saaios_cp_poke 2>/dev/null || true
  sleep 0.2
  if insmod "$KO" dry_run=0 read_only=1 \
      atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK \
      cp_phys=$PHYS ap_base=0x40200000 pcie_ch=0 \
      sig_hex=$SIG dump_n=16; then
    echo "PHYS=$PHYS RC=0" >> "$OUT"
  else
    echo "PHYS=$PHYS RC=$?" >> "$OUT"
  fi
  dmesg | grep saaios_cp_poke | tail -14 >> "$OUT"
  rmmod saaios_cp_poke 2>/dev/null || true
  sleep 0.3
done

echo "STATE_AFTER:$(cat /sys/devices/platform/cpif/modem_state)" >> "$OUT"
echo "rmnet0_rx=$(cat /sys/class/net/rmnet0/statistics/rx_packets 2>/dev/null)" >> "$OUT"
echo DONE >> "$OUT"
cat "$OUT"
