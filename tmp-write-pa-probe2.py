#!/usr/bin/env python3
from pathlib import Path
Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/pa-probe2.sh").write_text(
    "#!/bin/sh\n"
    "OUT=/data/saaios/var/pa-probe2.txt\n"
    "mkdir -p /data/saaios/var\n"
    'echo start > "$OUT"\n'
    'echo STATE:$(cat /sys/devices/platform/cpif/modem_state) >> "$OUT"\n'
    "ATU=$(grep exynos_pcie_rc_set_outbound_atu /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)\n"
    "L1=$(grep s51xx_pcie_l1ss_ctrl /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)\n"
    "LK=$(grep exynos_pcie_rc_chk_link_status /proc/kallsyms | grep ' t ' | head -1 | cut -d' ' -f1)\n"
    'echo ATU:$ATU L1:$L1 LK:$LK >> "$OUT"\n'
    "i=0; while [ $i -lt 6 ]; do\n"
    "  /data/saaios/bin/sit-sim-status query-sim-status >/dev/null\n"
    "  /data/saaios/bin/sit-sim-status query-radio-state >/dev/null\n"
    "  i=$((i+1))\n"
    "done\n"
    "rmmod saaios_cp_poke 2>/dev/null\n"
    "insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 "
    "atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK "
    "cp_phys=0x87200000 ap_base=0x40200000 pcie_ch=0\n"
    'echo RO_BTL:$? >> "$OUT"\n'
    "dmesg | grep saaios_cp_poke | tail -25 >> \"$OUT\"\n"
    "rmmod saaios_cp_poke 2>/dev/null\n"
    "i=0; while [ $i -lt 4 ]; do\n"
    "  /data/saaios/bin/sit-sim-status query-sim-status >/dev/null\n"
    "  i=$((i+1))\n"
    "done\n"
    "insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 "
    "atu_fn=0x$ATU l1ss_fn=0x$L1 link_fn=0x$LK "
    "cp_phys=0x814f47f4 ap_base=0x40200000 pcie_ch=0\n"
    'echo RO_HYP:$? >> "$OUT"\n'
    "dmesg | grep saaios_cp_poke | tail -25 >> \"$OUT\"\n"
    "rmmod saaios_cp_poke 2>/dev/null\n"
    'echo STATE:$(cat /sys/devices/platform/cpif/modem_state) >> "$OUT"\n'
    'echo DONE >> "$OUT"\n'
)
print("ok")
