#!/usr/bin/env python3
from pathlib import Path
p = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/pa-hard.sh")
lines = [
    "#!/bin/sh",
    "OUT=/data/saaios/var/pa-hard.txt",
    'echo start > "$OUT"',
    "rm -f /run/saaios-sit-status.lock",
    "rmmod saaios_cp_poke 2>/dev/null",
    "/data/saaios/bin/sit-sim-status query-radio-state >/dev/null",
    "/data/saaios/bin/sit-sim-status query-sim-status >/dev/null",
    "insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 "
    "atu_fn=0xffffffe4677b3484 l1ss_fn=0xffffffe467d36408 link_fn=0xffffffe4677b8e24 "
    "cp_phys=0x87200000 ap_base=0x40200000 pcie_ch=0",
    'echo RO_BTL:$? >> "$OUT"',
    'dmesg | grep saaios_cp_poke | tail -15 >> "$OUT"',
    "rmmod saaios_cp_poke 2>/dev/null",
    "/data/saaios/bin/sit-sim-status query-sim-status >/dev/null",
    "insmod /data/saaios/saaios_cp_poke.ko dry_run=0 read_only=1 "
    "atu_fn=0xffffffe4677b3484 l1ss_fn=0xffffffe467d36408 link_fn=0xffffffe4677b8e24 "
    "cp_phys=0x814f47f4 ap_base=0x40200000 pcie_ch=0",
    'echo RO_HYP:$? >> "$OUT"',
    'dmesg | grep saaios_cp_poke | tail -15 >> "$OUT"',
    "rmmod saaios_cp_poke 2>/dev/null",
    'cat /sys/devices/platform/cpif/modem_state >> "$OUT"',
    'echo DONE >> "$OUT"',
]
p.write_text("\n".join(lines) + "\n", newline="\n")
raw = p.read_bytes()[:12]
print(raw.hex(), p.stat().st_size)
