#!/usr/bin/env python3
"""RO: TEST_SIM / VirtualSim / TestSim / CampOn — any SIT opcode + GET_APP bypass?"""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def nearby_strings(off, radius=0x180):
    lo = max(0, off - radius)
    hi = min(len(IMG), off + radius)
    chunk = IMG[lo:hi]
    out = []
    i = 0
    while i < len(chunk):
        if 32 <= chunk[i] < 127:
            j = i
            while j < len(chunk) and 32 <= chunk[j] < 127:
                j += 1
            if j - i >= 5:
                out.append((lo + i, chunk[i:j].decode("ascii", "replace")))
            i = j + 1
        else:
            i += 1
    return out


targets = [
    (b"TEST_SIM", 0xCBE74F),
    (b"TestSim", 0x2E4A220),
    (b"test SIM", 0x4E2CCCB),
    (b"VirtualSim", 0x102D3FC),
    (b"ENG_MODE", 0x1028E49),
    (b"CampOn", 0xC82A60),
    (b"CAMP_ON", 0x10C74B1),
    (b"NS_ATTACH", 0xCCE14E),
    (b"StartNetwork", 0xCD1D0F),
]

for needle, hint in targets:
    off = IMG.find(needle) if hint < 0 else hint
    # prefer exact
    off = IMG.find(needle)
    print(f"\n######## {needle!r} @{hex(off)} ########")
    for a, s in nearby_strings(off)[:40]:
        mark = " <<" if a == off else ""
        print(f"  {hex(a)}: {s}{mark}")

# Also search SIT-ish id patterns near VirtualSim / TEST_SIM function names
print("\n=== more VirtualSim / TEST_SIM occurrences (full strings) ===")
for n in [b"VirtualSim", b"TEST_SIM", b"TestSimStatus", b"IsTestSim", b"IsVirtualSim",
          b"ForceCamp", b"force_camp", b"FakeSim", b"EngineeringMode",
          b"SIT_NET", b"OEM_HOOK", b"OemHookRaw", b"RequestOemHook"]:
    off = 0
    c = 0
    while c < 8:
        i = IMG.find(n, off)
        if i < 0:
            break
        s = IMG[i : i + 80].split(b"\x00")[0]
        print(f"  {n!r}@{hex(i)}: {s!r}")
        off = i + 1
        c += 1

print("DONE")
