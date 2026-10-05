#!/usr/bin/env python3
import subprocess
from pathlib import Path

stream = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so")
lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")

# Disassemble Init - look for remapping of state/pin bytes
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x666a0","--stop-address=0x668d0", str(stream)
], text=True, errors="replace"))

# GetPinState
print("=== GetPinState ===")
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x66a20","--stop-address=0x66a80", str(stream)
], text=True, errors="replace"))

# BuildRilCardStatusApplications - pin vs state copy
print("=== BuildRilCardStatusApplications (first part) ===")
dump = subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x154f00","--stop-address=0x155200", str(lib)
], text=True, errors="replace")
for line in dump.splitlines():
    if any(x in line for x in ("ldrb", "strb", "ldr\tw", "str\tw", "cmp\tw", "bl\t", "mov\tw")):
        print(line)
