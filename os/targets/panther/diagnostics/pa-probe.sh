#!/bin/sh
OUT=/data/saaios/var/pa-probe.txt
mkdir -p /data/saaios/var
echo start > "$OUT"
echo STATE:$(cat /sys/devices/platform/cpif/modem_state) >> "$OUT"
ATU=$(grep exynos_pcie_rc_set_outbound_atu /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
L1=$(grep s51xx_pcie_l1ss_ctrl /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
LK=$(grep exynos_pcie_rc_chk_link_status /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)
echo ATU:$ATU L1:$L1 LK:$LK >> "$OUT"
/data/saaios/bin/sit-sim-status query-sim-status >/dev/null
/data/saaios/bin/sit-sim-status query-radio-state >/dev/null
insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK cp_phys=0x814f47f4 ap_base=0x40200000 pcie_ch=0
echo RO_PA:$? >> "$OUT"
dmesg | grep saaios_cp_poke | tail -20 >> "$OUT"
rmmod saaios_cp_poke 2>/dev/null
/data/saaios/bin/sit-sim-status query-radio-state >/dev/null
insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK cp_phys=0x87200000 ap_base=0x40200000 pcie_ch=0
echo RO_BTL:$? >> "$OUT"
dmesg | grep 'saaios_cp_poke: RO' | tail -8 >> "$OUT"
rmmod saaios_cp_poke 2>/dev/null
echo STATE:$(cat /sys/devices/platform/cpif/modem_state) >> "$OUT"
echo DONE >> "$OUT"
