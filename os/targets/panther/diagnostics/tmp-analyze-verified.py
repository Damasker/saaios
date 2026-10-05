#!/usr/bin/env python3
"""Gate: Pin1Verified vs PIN_DISABLED FCP; xref setter BL 0x19916d2 callers with #2/#5."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
VA_BASE = 0x40010000
img = PATH.read_bytes()
SET_APP = 0x19916D2  # candidate app_state writer from STATUS_UPD


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8, rd


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


def find_bl_to(target, window_start, window_end):
    hits = []
    for o in range(window_start & ~1, window_end - 4, 2):
        t = bl_target(o)
        if t == target:
            hits.append(o)
    return hits


# Scan MAIN for BL -> SET_APP, look back 8 bytes for MOVS #2 or #5
print(f"=== callers of set_app? BL->{hex(SET_APP)} va={hex(va(SET_APP))} ===")
pin_writers = []
ready_writers = []
other = []
end = MAIN_OFF + 0x05917ACC
for o in range(MAIN_OFF, end - 4, 2):
    t = bl_target(o)
    if t != SET_APP:
        continue
    # look back up to 12 bytes for MOVS Rd, #imm
    imm = None
    for b in range(2, 16, 2):
        hw = u16(o - b)
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            break
    rec = (o, imm)
    if imm == 2:
        pin_writers.append(rec)
    elif imm == 5:
        ready_writers.append(rec)
    else:
        other.append(rec)

print(f"PIN(2) writers via this BL: {len(pin_writers)}")
for o, imm in pin_writers[:12]:
    print(f"  BL@0x{o:x}/va0x{va(o):x} MOVS#{imm}")
print(f"READY(5) writers via this BL: {len(ready_writers)}")
for o, imm in ready_writers[:12]:
    print(f"  BL@0x{o:x}/va0x{va(o):x} MOVS#{imm}")
print(f"other imm counts: {len(other)}")
from collections import Counter
print("  imm hist:", Counter(i for _, i in other).most_common(10))

# Pin1Verified related strings + log ids
print("\n=== Pin1Verified / IsSimVerifyComplete strings ===")
for n in [
    b"Pin1Verified",
    b"IsSimVerifyComplete",
    b"MePerVerified",
    b"SetPin1Verified",
    b"pin1_verified",
    b"PIN1 Verified",
    b"PIN1 verified",
    b"Pin Verified",
]:
    start = 0
    while True:
        j = img.find(n, start)
        if j < 0:
            break
        s0 = j
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = j
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        hid = u32(s0 - 8) if s0 >= 8 else 0
        print(f"  0x{s0:x} id?=0x{hid & 0xffff:x}: {img[s0:s1].decode()[:140]}")
        start = j + 1

# Does PIN_DISABLED FCP site @0x2b36782 call SET_APP?
print("\n=== BLs near FCP PIN_DISABLED USIM @0x2b36700..0x2b36900 ===")
for o in range(0x2B36700, 0x2B36900, 2):
    t = bl_target(o)
    if t:
        mark = ""
        if t == SET_APP:
            mark = "  <<SET_APP"
        if t == 0x20E184E:
            mark = "  <<other_setter?"
        print(f"  BL@0x{o:x} -> 0x{t:x}{mark}")

# Does STATUS_UPD READY path require CMP after Pin1Verified read?
print("\n=== STATUS_UPD READY path detail @0x14fb580..0x14fb5d0 ===")
o = 0x14FB580
while o < 0x14FB5D0:
    t = bl_target(o)
    r = movw(o)
    hw = u16(o)
    if t:
        print(f"  0x{o:x}: BL -> 0x{t:x}" + (" SET_APP" if t == SET_APP else ""))
        o += 4
    elif r:
        print(f"  0x{o:x}: MOVW r{r[1]},#0x{r[0]:x}")
        o += 4
    elif (hw & 0xFF00) == 0x2000:
        print(f"  0x{o:x}: MOVS r{(hw>>8)&7},#{hw&0xff}")
        o += 2
    elif (hw & 0xFF00) == 0x2800:
        print(f"  0x{o:x}: CMP r{(hw>>8)&7},#{hw&0xff}")
        o += 2
    elif (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        o += 4
    else:
        o += 2

# PIN SKIP: confirm eSIM gate only — dump around first 0x11d6 site
print("\n=== first PIN_SKIP_NOT_eSIM site ===")


def isolated(imm, lim=3):
    hits = []
    for o in range(MAIN_OFF, end - 8, 2):
        r = movw(o)
        if not r or r[0] != imm:
            continue
        fol = False
        for p in range(o + 4, o + 40, 2):
            r2 = movw(p)
            if r2 and r2[0] == imm + 1:
                fol = True
                break
        if not fol:
            hits.append(o)
            if len(hits) >= lim:
                break
    return hits


for o in isolated(0x11D6, 3):
    print(f"site 0x{o:x} va=0x{va(o):x}")
    # search back for CMP / MOVS and string siblings 0x11ce Invalid SimState, 0x11b4 MISMATCH
    for p in range(o - 0x100, o + 0x40, 2):
        r = movw(p)
        if r and r[0] in (0x11B4, 0x11CE, 0x11D6, 0x11D7):
            print(f"  0x{p:x}: MOVW #0x{r[0]:x}")
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3, 4, 5):
            print(f"  0x{p:x}: MOVS #{hw&0xff}")

# Confirm no SIT opcode string for pin skip in MAIN SIT tables
print("\n=== SIT_*PIN* strings ===")
for n in [b"SIT_SIM_PIN", b"SIT_PIN", b"sitTxPinSkip", b"sitPinSkip", b"PIN_SKIP"]:
    j = 0
    c = 0
    while c < 6:
        i = img.find(n, j)
        if i < 0:
            break
        s0 = i
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = i
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        print(f"  0x{s0:x}: {img[s0:s1].decode()[:100]}")
        c += 1
        j = i + 1
