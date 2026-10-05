#!/usr/bin/env python3
import subprocess
p = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
ranges = [
    ("IsLegacy", 0x636F0, 0x63710),
    ("GetNum", 0x63700, 0x63710),
    ("GetCardState", 0x63710, 0x63750),
    ("GetSlotState", 0x63750, 0x637B0),
    ("GetAtrSize", 0x637B0, 0x63890),
    ("fillFromModem_head", 0x624E0, 0x62640),
    ("Init_head", 0x63020, 0x63120),
]
for name, start, stop in ranges:
    print("===", name)
    out = subprocess.check_output(
        [
            "aarch64-linux-gnu-objdump",
            "-d",
            f"--start-address={start:#x}",
            f"--stop-address={stop:#x}",
            p,
        ],
        text=True,
        errors="replace",
    )
    print(out)
