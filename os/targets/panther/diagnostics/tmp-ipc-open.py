#!/usr/bin/env python3
from pathlib import Path
import subprocess
import struct

lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so").read_bytes()
# format strings
for n in (b"umts_ipc%d", b"umts_ipc%u", b"/dev/umts_ipc", b"umts_rfs%d", b"/dev/umts_rfs",
          b"ipc%d", b"RIL_SOCKET_%d", b"socket id", b"SocketId=%d", b"phoneId=%d",
          b"RIL[%d]", b"ril-%d", b"MultiSim"):
    j = lib.find(n)
    print(n, hex(j) if j>=0 else None)
    if j is not None and j >= 0:
        end = lib.find(b"\0", j)
        print(" ", lib[j:end][:120])

# IoChannel::Open and Ctor
print(subprocess.check_output([
    "aarch64-linux-gnu-objdump","-d",
    "--start-address=0x1d5a00","--stop-address=0x1d5c20",
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
], text=True, errors="replace"))

# Who calls IoChannelC1 - find bl to 0x1d5a00
# Scan for BL immediate targeting 1d5a00
text_off = 0xf7000
text_vma = 0xf7000
# get size from earlier ~ 
# Search BL: 100101s imm26
target = 0x1d5a00
hits = []
i = 0
text = lib[text_off:text_off+0x100000]  # generous
# actually use full .text size from objdump
import re
hdr = subprocess.check_output(["aarch64-linux-gnu-objdump","-h",
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"], text=True)
for line in hdr.splitlines():
    parts=line.split()
    if len(parts)>=6 and parts[1]=='.text':
        text_size=int(parts[2],16)
text = lib[text_off:text_off+text_size]
for i in range(0, len(text)-4, 4):
    w = struct.unpack_from('<I', text, i)[0]
    if (w & 0xFC000000) != 0x94000000:
        continue
    imm26 = w & 0x3FFFFFF
    if imm26 & 0x2000000:
        imm26 -= 0x4000000
    pc = text_vma + i
    dest = pc + imm26 * 4
    if dest == target or dest == 0x1d5a00:  # C1/C2 same
        hits.append(pc)
print('IoChannel ctor callsites', [hex(h) for h in hits[:30]], 'count', len(hits))
for h in hits[:15]:
    print('---', hex(h))
    print(subprocess.check_output([
        "aarch64-linux-gnu-objdump","-d",
        f"--start-address={h-0x30:#x}", f"--stop-address={h+0x40:#x}",
        "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
    ], text=True, errors="replace")[-1200:])
