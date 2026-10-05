#!/usr/bin/env python3
from pathlib import Path
IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
for off in (0x14B7D50, 0x14B7D20, 0x14B7D7C):
    end = IMG.find(b"\x00", off)
    print(hex(off), IMG[off:end].decode("ascii", "replace"))
