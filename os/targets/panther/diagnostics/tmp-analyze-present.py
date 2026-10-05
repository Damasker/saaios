#!/usr/bin/env python3
"""Find Present flag writers (value 2); map Pin1Verified→Present; FCP effect on Present."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


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


# In STATUS: after PIN path STRB Pin1Verified to +0xBF5; check MePer and all STRB.W #0xBFx
print("=== All STRB.W to offsets 0xBF0..0xBFF in MAIN ===")
for o in range(MAIN_OFF, END - 4, 2):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) != 0xF880:
        continue
    imm = hw2 & 0xFFF
    if 0xBF0 <= imm <= 0xBFF:
        rt = (hw2 >> 12) & 0xF
        rn = hw & 0xF
        # preceding MOVS into rt
        val = None
        for b in range(2, 24, 2):
            h = u16(o - b)
            if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt:
                val = h & 0xFF
                break
            if (h & 0xFFC0) == 0x4600:  # MOV low
                # MOV rt, rm — track if prior MOVS
                pass
        print(f"  STRB.W r{rt},[r{rn},#{imm:#x}] @{o:#x}/va{va(o):#x} prev_imm={val}")

# Dump more of STATUS around Pin1Verified store and any Present updates
print("\n=== STATUS STRB.W cluster 0x14fb370..0x14fb4a0 ===")
o = 0x14FB370
while o < 0x14FB4A0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        if (hw & 0xFFF0) == 0xF880:
            extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xFFF0) == 0xF890:
            extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            extra = f" ;BL->{bl_target(o):#x}"
        elif movw(o):
            r = movw(o)
            extra = f" ;MOVW r{r[1]},#{r[0]:#x}"
        if (hw & 0xFF00) == 0xFB00 and (hw2 & 0xF0) == 0:
            rn, ra, rd, rm = hw & 0xF, (hw2 >> 12) & 0xF, (hw2 >> 8) & 0xF, hw2 & 0xF
            if ra == 0xF:
                extra = f" ;MUL r{rd},r{rn},r{rm}"
            else:
                extra = f" ;MLA r{rd},r{rn},r{rm},r{ra}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS #{hw&0xff}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        print(f"  {o:#x}: {hw:04x}{extra}")
        o += 2

# Search strings about Present
print("\n=== Present-related strings ===")
for n in [
    b"Present:%d",
    b"IsPresent",
    b"SimPresent",
    b"SetPresent",
    b"present =",
    b"Present =",
    b"m_Present",
    b"bPresent",
    b"Pin1Verified",
    b"MePerVerified",
    b"SimStatusType",
    b"SIM_STATUS",
]:
    idx = 0
    while True:
        j = img.find(n, idx)
        if j < 0:
            break
        s0 = j
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = j
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        print(f"  {s0:#x}: {img[s0:s1].decode()[:140]}")
        idx = j + 1
        if idx > j + 1000000:
            break

# Who calls setter that might update Present? Search STRB #0 (Present) near Pin1Verified writers
print("\n=== Near Pin1Verified STRB#20: any STRB of #2 to nearby offsets ===")
for site in (0x1F04576, 0x1F046A4):
    for p in range(site - 0x100, site + 0x200, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for q in range(p + 2, p + 16, 2):
                h = u16(q)
                if (h & 0xF800) == 0x7000:
                    print(f"  @{site:#x} vicinity: MOVS#2 + STRB [r{(h>>3)&7},#{(h>>6)&0x1f}] @{p:#x}")
                if (h & 0xFFF0) == 0xF880:
                    print(f"  @{site:#x} vicinity: MOVS#2 + STRB.W off={u16(q+2)&0xfff:#x} @{p:#x}")
        if (hw & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) in (0xBF4, 0xBF5, 0xBF6, 0xBF7, 0, 1, 2):
            print(f"  @{site:#x} vicinity: STRB.W off={u16(p+2)&0xfff:#x} @{p:#x}")

# After Pin1Verified=1, does code call STATUS update / DetermineSimStatus?
print("\n=== After 0x18e Pin1Verified sites: BL targets (next 8) ===")
for site in (0x1F04578, 0x1F046A6):  # after STRB
    count = 0
    o = site
    while count < 12 and o < site + 0x80:
        b = bl_target(o)
        if b is not None and (u16(o) & 0xF800) == 0xF000:
            print(f"  @{o:#x} BL->{b:#x}/va{va(b):#x}")
            count += 1
            o += 4
            continue
        if (u16(o) & 0xF800) in (0xE800, 0xF000, 0xF800):
            o += 4
        else:
            o += 2

# FCP window: does it store to Present field or call into STATUS?
# Known FCP body around 0x1e0c2fe — look for BL to STATUS fn or STRB Present-like
print("\n=== FCP 0x7ab @0x1e0c2fe function: STRB.W 0xBFx / MOVS#2 / BL to STATUS cluster ===")
# find function start
fn = 0x1E0C000
for p in range(0x1E0C2FE, 0x1E0C2FE - 0x400, -2):
    if u16(p) in (0xB570, 0xB5F0, 0xB5B0, 0xE92D):
        fn = p
        break
print(f"  fn guess {fn:#x}")
status_like = []
for o in range(fn, min(fn + 0x800, END - 4), 2):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and 0xBF0 <= (hw2 & 0xFFF) <= 0xBFF:
        status_like.append(f"STRB.W #{hw2&0xfff:#x} @{o:#x}")
    b = bl_target(o)
    if b and 0x14FB200 <= b <= 0x14FB700:
        status_like.append(f"BL STATUS_cluster @{o:#x}->{b:#x}")
    if b == 0x14FB6A0 or (b and abs(b - 0x14FB400) < 0x400):
        status_like.append(f"BL near STATUS @{o:#x}->{b:#x}")
print("  hits:", status_like[:20])

# Search who stores MOVS #2 then STRB to [rN, #0] where that might be Present
# Better: find all STRB.W to #0xBF6 with what register content — already know only Present copy
# Find writers of the source Present at whatever object [r5] is in STATUS

# Trace where r5 comes from at STATUS entry
print("\n=== Walk back STATUS fn for r5 origin / Present source ===")
# find PUSH at start of function containing 0x14fb380
for p in range(0x14FB380, 0x14FB380 - 0x600, -2):
    hw = u16(p)
    if hw in (0xB5F0, 0xB570, 0xB5F8, 0xB5B0, 0xB580) or (hw & 0xFF00) == 0xB500:
        # check e92d
        print(f"  possible PUSH @{p:#x} hw={hw:04x}")
    if (hw & 0xFFF0) == 0xE92D:
        print(f"  PUSH.W @{p:#x}")
        break
