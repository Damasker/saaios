#!/usr/bin/env python3
import subprocess
from pathlib import Path

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
data = lib.read_bytes()

# Find xrefs-ish: string offsets for umts_ipc0/1, then search ADRP patterns is hard;
# instead search for nearby code that loads the string via adrp/add by finding
# the string in .rodata and looking who uses it via readelf -x or nm.

# Use objdump -T / strings with context via llvm? Simpler: find "umts_ipc" in
# functions by dumping Io / ModemControl / Protocol related.

out = subprocess.check_output(["aarch64-linux-gnu-readelf","-sW",str(lib)], text=True, errors="replace")
keys = ("ModemIo", "IoChannel", "SitChannel", "OpenDevice", "ModemControl",
        "ProtocolChannel", "SendRaw", "WriteRequest", "IpcOpen", "RfsOpen",
        "GetSocketId", "RilSocket", "CreateContext", "PhoneContext")
for line in out.splitlines():
    if any(k in line for k in keys):
        print(line)

# Also search binary for both ipc0 and ipc1 near same function via string offsets
for name in (b"umts_ipc0", b"umts_ipc1", b"/dev/umts_ipc0", b"/dev/umts_ipc1"):
    j = data.find(name)
    print("str", name, hex(j) if j>=0 else None)

# Disassemble GetRilSocketId and RilContext constructors
print("==== GetRilSocketId ServiceInterface ====")
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x115c20","--stop-address=0x115cc0", str(lib)
], text=True, errors="replace"))

print("==== RilContextWrapper GetRilSocketId ====")
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x109780","--stop-address=0x109790", str(lib)
], text=True, errors="replace"))
