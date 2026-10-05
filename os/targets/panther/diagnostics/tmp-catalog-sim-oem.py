#!/usr/bin/env python3
from pathlib import Path
import struct

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA0, MAIN = 0x40010000, 0x16C10


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def cstr(o, n=48):
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if c == 0:
            break
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else "?"


print("=== catalog SIM OEM bank ===")
for o in range(0x6DE500, 0x6DE8C0, 28):
    mid = u16(o + 2)
    if 0x2F00 <= mid <= 0x2FFF:
        nva = u32(o + 4)
        noff = nva - VA0 + MAIN
        name = cstr(noff) if 0 <= noff < len(img) else hex(nva)
        print(
            f"@{hex(o)} flags={u16(o)} id={hex(mid)} meta={hex(u32(o+8))} "
            f"rsp={hex(u16(o+0xc))} +10={hex(u32(o+0x10))} +14={hex(u32(o+0x14))} "
            f"+18={u32(o+0x18)} {name}"
        )

for label, needle in [
    ("SIM_INIT_REQ", b"SIM_INIT_REQ\x00"),
    ("SIM_VERIFYPIN_REQ", b"SIM_VERIFYPIN_REQ\x00"),
    ("SIM_INFO_REQ", b"SIM_INFO_REQ\x00"),
]:
    j = img.find(needle)
    print(f"{label} str@{hex(j) if j>=0 else -1}")
    if j < 0:
        continue
    va = VA0 + (j - MAIN)
    pat = struct.pack("<I", va)
    pos = 0
    while True:
        h = img.find(pat, pos)
        if h < 0:
            break
        base = h - 4
        if 0x6DE000 <= base <= 0x6E1000:
            print(
                f"  catalog @{hex(base)} flags={u16(base)} id={hex(u16(base+2))} "
                f"meta={hex(u32(base+8))} rsp={hex(u16(base+0xc))} +18={u32(base+0x18)}"
            )
        pos = h + 1
