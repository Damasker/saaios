#!/usr/bin/env python3
from pathlib import Path
import struct
import sys
sys.path.insert(0, "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
from importlib.machinery import SourceFileLoader
mod = SourceFileLoader(
    "gate",
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/tmp-reverify-set5-gate.py",
).load_module()
p = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/"
    "os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = memoryview(p.read_bytes())
print("=== post-READY window 0x14fb630..0x14fb6a0 ===")
for off, s in mod.decode_thumb_window(img, 0x14FB630, 0x14FB6A0):
    print(f"  0x{off:x}: {s}")
print("=== STATUS head BNE after CMP#4 (raw halfwords 0x14fb34c..0x14fb358) ===")
for o in range(0x14FB34C, 0x14FB358, 2):
    print(f"  0x{o:x}: {struct.unpack_from('<H', img, o)[0]:04x}")
