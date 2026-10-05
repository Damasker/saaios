#!/usr/bin/env python3
"""START_NETWORK / camp / attach GET_APP gates; PIN(2) / Pin1Verified accept?"""
import struct
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()

def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]

def bl_target(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32

needles = [
    b"START_NETWORK Ignored",
    b"START_NETWORK",
    b"Stop other stack",
    b"SIM is not ready",
    b"Not camped",
    b"NS_START",
]
print("=== strings ===")
hits = {n: [] for n in needles}
off = 0
data = IMG
while True:
    i = data.find(b"START_NETWORK", off)
    if i < 0:
        break
    ctx = data[i : i + 80].split(b"\x00", 1)[0]
    print(f"  {hex(i)} {ctx[:70]!r}")
    off = i + 1
    if off > 40:
        break  # first few
# all START_NETWORK
print("-- all START_NETWORK offs --")
off = 0
n = 0
while n < 15:
    i = data.find(b"START_NETWORK", off)
    if i < 0:
        break
    ctx = data[i : i + 64].split(b"\x00", 1)[0]
    print(f"  {hex(i)} {ctx!r}")
    off = i + 1
    n += 1

print("\n=== SIM is not ready ===")
off = 0
n = 0
while n < 12:
    i = data.find(b"SIM is not ready", off)
    if i < 0:
        break
    ctx = data[max(0, i - 24) : i + 40].split(b"\x00")
    print(f"  {hex(i)}")
    off = i + 1
    n += 1

GET_APP = 0x18EC8C0
print("\n=== BL GET_APP then CMP #1/2/4/5 in +/-0x80 (sample first 40 BLs) ===")
bls = []
# restricted scan around known NET/SIM ranges to stay fast
ranges = [(0x18E0000, 0x1900000), (0x14C0000, 0x1500000), (0x2600000, 0x2620000)]
for lo, hi in ranges:
    o = lo
    while o < hi - 4:
        t = bl_target(o)
        if t == GET_APP:
            bls.append(o)
        o += 2
print("  GET_APP BLs", len(bls), [hex(x) for x in bls[:25]])

def cmps_after(site, nbytes=0x60):
    found = []
    o, end = site + 4, site + nbytes
    while o < end:
        hw = u16(o)
        if (hw & 0xFF00) == 0x2800:
            found.append((hex(o), hw & 0xFF))
        o += 2
    return found

print("  CMP imm after each GET_APP BL:")
for s in bls:
    c = cmps_after(s)
    interesting = [x for x in c if x[1] in (1, 2, 4, 5, 6, 7)]
    if interesting:
        print(f"    {hex(s)} {interesting}")

print("DONE")
