#!/usr/bin/env python3
"""Map SIT_SET_ENG_MODE / VirtualSim opcode IDs; check START_NETWORK bypass."""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
LIB = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
).read_bytes()
BASE = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-base.so"
).read_bytes()


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def nearby(data, off, radius=0x200):
    lo = max(0, off - radius)
    hi = min(len(data), off + radius)
    chunk = data[lo:hi]
    out = []
    i = 0
    while i < len(chunk):
        if 32 <= chunk[i] < 127:
            j = i
            while j < len(chunk) and 32 <= chunk[j] < 127:
                j += 1
            if j - i >= 6:
                out.append((lo + i, chunk[i:j].decode("ascii", "replace")))
            i = j + 1
        else:
            i += 1
    return out


# 1) All SIT_* near ENG / Virtual / TEST / FORCE / CAMP in MAIN name table
print("=== MAIN SIT_* names of interest ===")
for n in [
    b"SIT_SET_ENG_MODE",
    b"SIT_GET_ENG_MODE",
    b"SIT_SET_IMS_TEST_MODE",
    b"SIT_SET_EMC_MODE",
    b"SIT_SET_RF_DESENSE_MODE",
    b"SIT_EXEC_RADIO_NODE",
    b"SIT_SET_RADIO_NODE",
    b"VIRTUAL_SIM",
    b"VirtualSim",
    b"SIT_SET_VIRTUAL",
    b"SIT_VIRTUAL",
    b"SIT_OEM",
    b"OEM_HOOK",
]:
    off = 0
    while True:
        i = IMG.find(n, off)
        if i < 0:
            break
        s = IMG[i : i + 80].split(b"\x00")[0]
        print(f"  {hex(i)} {s!r}")
        off = i + 1
        if off - i > 0x100000:  # safety
            break
        if sum(1 for _ in [0]) and off > i + 1:
            # limit per needle
            count = 1
            while count < 5:
                j = IMG.find(n, off)
                if j < 0:
                    break
                s = IMG[j : j + 80].split(b"\x00")[0]
                print(f"  {hex(j)} {s!r}")
                off = j + 1
                count += 1
            break

# Dump contiguous SIT_ name table around SIT_SET_ENG_MODE with possible ID enum nearby
eng = IMG.find(b"SIT_SET_ENG_MODE")
print(f"\n=== ENG_MODE name table neighborhood @{hex(eng)} ===")
for a, s in nearby(IMG, eng, 0x300):
    if s.startswith("SIT_") or "ENG" in s or "TEST" in s or "VIRTUAL" in s.upper():
        print(f"  {hex(a)}: {s}")

# 2) Factory SO strings
print("\n=== factory SO: eng/virtual/test/oem builders ===")
for label, data in [("stream", STREAM), ("sitril", LIB), ("base", BASE)]:
    print(f"-- {label} --")
    for n in [
        b"EngMode",
        b"ENG_MODE",
        b"SetEng",
        b"VirtualSim",
        b"VIRTUAL_SIM",
        b"OemHook",
        b"OEM_HOOK",
        b"TestMode",
        b"IMS_TEST",
        b"EmcMode",
        b"RF_DESENSE",
        b"RadioNode",
        b"ForceCamp",
        b"FakeSim",
        b"LabMode",
        b"BuildSetEng",
        b"BuildVirtual",
        b"BuildOem",
    ]:
        off = 0
        c = 0
        while c < 5:
            i = data.find(n, off)
            if i < 0:
                break
            s = data[i : i + 100].split(b"\x00")[0]
            if len(s) >= 4:
                print(f"  {hex(i)} {s[:90]!r}")
            off = i + 1
            c += 1

# 3) SIT opcode tables often: string ptr then halfword id. Search lit refs to SIT_SET_ENG_MODE
print(f"\n=== lit refs to SIT_SET_ENG_MODE VA={hex(eng)} ===")
pat = struct.pack("<I", eng)
refs = []
start = 0
while len(refs) < 20:
    j = IMG.find(pat, start)
    if j < 0:
        break
    if abs(j - eng) > 4:
        refs.append(j)
    start = j + 1
print("refs", [hex(r) for r in refs])
for r in refs[:8]:
    # dump nearby words
    words = [hex(u16(IMG, r + k)) for k in range(-16, 16, 2)]
    print(f"  @{hex(r)} u16s: {words}")
    # also u32 around
    for k in range(-0x20, 0x20, 4):
        v = struct.unpack_from("<I", IMG, r + k)[0]
        if 0x0100 <= v <= 0x0FFF:
            print(f"    word@{hex(r+k)} = {hex(v)} (sit-id-ish)")

# VirtualSim file names → find SIT_* VirtualSim request names
print("\n=== VirtualSim SIT_* strings ===")
for n in [b"SIT_", b"VIRTUAL", b"Virtual"]:
    pass
for frag in [
    b"SIT_SET_VIRTUAL_SIM",
    b"SIT_GET_VIRTUAL_SIM",
    b"SIT_VIRTUAL_SIM",
    b"SIT_IND_VIRTUAL",
    b"VIRTUAL_SIM_STATUS",
    b"SIT_VSIM",
    b"SIT_SET_VSIM",
    b"SIT_ACTIVATE_VSIM",
]:
    i = IMG.find(frag)
    print(f"  {frag!r}: {hex(i) if i>=0 else None}")
    if i >= 0:
        for a, s in nearby(IMG, i, 0x100):
            if "SIT_" in s or "VSIM" in s or "VIRTUAL" in s.upper():
                print(f"    {hex(a)}: {s}")

print("DONE")
