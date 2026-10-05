#!/usr/bin/env python3
"""Create patched COPY of probe-b modem.bin: STATUS SET#2 MOVS #2 -> #5.
Keeps TOC CRC (algorithm unknown). Prints new SHA-256. Backs up original name."""
from __future__ import annotations
import hashlib
import shutil
import struct
from pathlib import Path

src = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
backup = src.with_name("saaios-probe-b-modem.bin.STOCK_BAK")
dst = Path("/mnt/c/Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin")
dst.parent.mkdir(parents=True, exist_ok=True)

if not backup.exists():
    shutil.copy2(src, backup)
    print("backup", backup)

data = bytearray(src.read_bytes())
off = 0x14FB404
assert data[off] == 0x02 and data[off + 1] == 0x20, data[off : off + 2].hex()
# Thumb MOVS r0,#imm8 little-endian: low byte=imm, high=0x20
data[off] = 0x05  # MOVS r0,#5 READY
assert data[off] | (data[off + 1] << 8) == 0x2005

# TOC MAIN crc left unchanged (unknown Shannon CRC; UDL may reject — then restore)
sha = hashlib.sha256(data).hexdigest()
dst.write_bytes(data)
stock_sha = hashlib.sha256(src.read_bytes()).hexdigest()
print("stock_sha", stock_sha)
print("patched_sha", sha)
print("patched_path", dst)
print("patch_site", hex(off), "2002->2005")
print("size", len(data))
