#!/usr/bin/env python3
"""RO: MemoryRead/Write / MEM_WRITE / DM_ENABLE context — proven DM mem poke?"""
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]


def dump_str(off, before=0x40, after=0x80):
    s = IMG[off : off + 64].split(b"\x00")[0]
    print(f"\n=== @{hex(off)} {s!r} ===")
    # surrounding strings
    lo = max(0, off - 0x100)
    chunk = IMG[lo : off + 0x200]
    # find nearby C strings
    i = 0
    while i < len(chunk):
        if 32 <= chunk[i] < 127:
            j = i
            while j < len(chunk) and 32 <= chunk[j] < 127:
                j += 1
            if j - i >= 4:
                abs_o = lo + i
                print(f"  str@{hex(abs_o)}: {chunk[i:j].decode('ascii', 'replace')}")
            i = j + 1
        else:
            i += 1


needles = [
    b"MemoryRead",
    b"MemoryWrite",
    b"MEM_WRITE",
    b"RegWrite",
    b"DM_ENABLE",
    b"MEM_READ",
    b"MemRead",
    b"MemWrite",
    b"Peek",
    b"Poke",
]
for n in needles:
    off = 0
    hits = 0
    while True:
        i = IMG.find(n, off)
        if i < 0:
            break
        hits += 1
        if hits <= 3:
            dump_str(i)
        off = i + 1
    print(f"TOTAL {n!r}: {hits}")

# Cross-ref: who loads pointer to MemoryWrite as lit?
print("\n=== lit-pool refs to MemoryWrite VA (file==VA?) ===")
mw = IMG.find(b"MemoryWrite")
mr = IMG.find(b"MemoryRead")
print(f"MemoryWrite@{hex(mw)} MemoryRead@{hex(mr)}")
# Search for little-endian VA words equal to these offsets (Shannon often VA=file for text; strings may be in data VA)
# Also search relative in 0x40xxxxxx if remapped - try raw file offsets as VAs
for name, va in [("MemoryWrite", mw), ("MemoryRead", mr), ("MEM_WRITE", IMG.find(b"MEM_WRITE")), ("DM_ENABLE", IMG.find(b"DM_ENABLE"))]:
    if va < 0:
        continue
    pat = struct.pack("<I", va)
    refs = []
    start = 0
    while len(refs) < 8:
        j = IMG.find(pat, start)
        if j < 0:
            break
        # skip self / string area
        if abs(j - va) > 4:
            refs.append(j)
        start = j + 1
    print(f"  lit refs to {name} VA={hex(va)}: {[hex(x) for x in refs]}")

# Check if MemoryRead/Write are Qualcomm DIAG-ish or Shannon DM table
# Look at 0x325000 area structure
print("\n=== around MemoryRead table ===")
base = mr & ~0xFFF
print(IMG[mr - 64 : mr + 128])

# DM_ENABLE context — enable protocol?
print("\n=== DM_ENABLE neighborhood strings only ===")
dme = IMG.find(b"DM_ENABLE")
print(IMG[dme - 200 : dme + 200].replace(b"\x00", b"|"))

print("DONE")
