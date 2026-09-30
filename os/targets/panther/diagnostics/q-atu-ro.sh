#!/bin/sh
OUT=/data/saaios/var/ob1-ro.txt
echo start > "$OUT"
rm -f /run/saaios-sit-status.lock
i=0
while [ $i -lt 5 ]; do
  /data/saaios/bin/sit-sim-status query-sim-status >/dev/null
  i=$((i+1))
done
echo WIDTH:$(cat /sys/bus/pci/devices/0000:01:00.0/current_link_width) >> "$OUT"
ATU=$(grep exynos_pcie_rc_set_outbound_atu /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
L1=$(grep s51xx_pcie_l1ss_ctrl /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
LK=$(grep exynos_pcie_rc_chk_link_status /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
rmmod saaios_cp_poke 2>/dev/null
# OB1 window: ioremap AP 0x40000000; ATU call still hits OB2 (harmless).
# Read doorbell base + TOC-like offset 0x10000.
for PHYS in 0x40000000 0x40000001 0x40000002 0x40000003 0x40010000 0x40010001 0x40010002 0x40010003 0x40010004 0x40010005 0x40010006 0x40010007; do
  insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK cp_phys=$PHYS ap_base=0x40000000 pcie_ch=0
  echo try_$PHYS:$? >> "$OUT"
  dmesg | grep 'saaios_cp_poke: RO' | tail -1 >> "$OUT"
  rmmod saaios_cp_poke 2>/dev/null
done
# Also OB2@0x40200000 with target BTL for contrast
insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK cp_phys=0x87200000 ap_base=0x40200000 pcie_ch=0
echo RO_BTL:$? >> "$OUT"
dmesg | grep 'saaios_cp_poke: RO' | tail -1 >> "$OUT"
rmmod saaios_cp_poke 2>/dev/null
echo STATE:$(cat /sys/devices/platform/cpif/modem_state) >> "$OUT"
/data/saaios/bin/sit-sim-status query-sim-status >> "$OUT" 2>&1
echo DONE >> "$OUT"
