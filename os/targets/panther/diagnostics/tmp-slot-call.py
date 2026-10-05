#!/usr/bin/env python3
import subprocess
p = "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
# Init continuation where fillSlotStatusDataFromModemData is called
out = subprocess.check_output(
    [
        "aarch64-linux-gnu-objdump",
        "-d",
        "--start-address=0x633a0",
        "--stop-address=0x63490",
        p,
    ],
    text=True,
    errors="replace",
)
print(out)
# also modern fill around slot_state / iccid / num apps
out2 = subprocess.check_output(
    [
        "aarch64-linux-gnu-objdump",
        "-d",
        "--start-address=0x626e0",
        "--stop-address=0x62880",
        p,
    ],
    text=True,
    errors="replace",
)
print(out2)
