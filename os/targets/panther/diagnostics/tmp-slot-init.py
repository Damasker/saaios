#!/usr/bin/env python3
import subprocess
p = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
out = subprocess.check_output(
    [
        "aarch64-linux-gnu-objdump",
        "-d",
        "--start-address=0x63020",
        "--stop-address=0x63280",
        p,
    ],
    text=True,
    errors="replace",
)
print(out)
# also more of fillFromModem around card/slot state stores
out2 = subprocess.check_output(
    [
        "aarch64-linux-gnu-objdump",
        "-d",
        "--start-address=0x625a0",
        "--stop-address=0x62780",
        p,
    ],
    text=True,
    errors="replace",
)
print(out2)
