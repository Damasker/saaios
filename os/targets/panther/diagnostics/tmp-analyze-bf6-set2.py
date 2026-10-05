#!/usr/bin/env python3
"""Find who writes Present/auth byte=2; VerifyPin aftermath; FCP vs Present enum."""
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


# Find STATUS entry - look for PUSH.W walking back from 0x14fb218
print("=== Find STATUS fn entry ===")
fn = None
for p in range(0x14FB218, 0x14FA800, -2):
    hw = u16(p)
    if (hw & 0xFFF0) == 0xE92D:
        fn = p
        print(f"PUSH.W @{p:#x} mask={u16(p+2):04x}")
        break
    if hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580, 0xB5B8, 0xB5F7):
        print(f"PUSH @{p:#x}={hw:04x}")
        fn = p
print(f"fn={fn:#x}" if fn else "fn not found")

# Callers of candidate entries around STATUS
for target in range(0x14FAE00, 0x14FB320, 2):
    # only check plausible entry points (PUSH)
    hw = u16(target)
    if not ((hw & 0xFFF0) == 0xE92D or hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580)):
        continue
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == target]
    if cs:
        print(f"  entry {target:#x} callers={len(cs)}: {[hex(c) for c in cs[:8]]}")

# Log 0x106c string (SET_APP related)
print("\n=== log 0x106c / 0x106b strings ===")
for lid in (0x106B, 0x106C, 0x106D):
    for off in range(0x4D4E000, 0x4D51000):
        w = struct.unpack_from("<I", img, off)[0]
        if (w & 0xFF) == 0x44 and ((w >> 8) & 0xFFFF) == lid:
            s = off + 8
            if 32 <= img[s] < 127:
                end = img.find(b"\0", s)
                print(f"  {lid:#x} @{s:#x}: {img[s:end].decode()[:160]}")
            break

# Search: STRB.W to #0xBF6 OR ADD #0xBF6 then STRB of immediate 2
# Also: any place that does MOVS #2 then STRB.W to [rN,#0] where rN points at Present trio
print("\n=== All MOVS #2 + STRB.W within same 16 bytes to off 0xBF4/5/6 ===")
for o in range(MAIN_OFF, END - 8, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000 or (hw & 0xFF) != 2:
        continue
    rt = (hw >> 8) & 7
    for q in range(o + 2, o + 20, 2):
        h = u16(q)
        if (h & 0xFFF0) == 0xF880:
            h2 = u16(q + 2)
            if ((h2 >> 12) & 0xF) == rt and (h2 & 0xFFF) in (0xBF4, 0xBF5, 0xBF6, 0, 1, 2):
                print(f"  MOVS#2 STRB.W [r{h&0xf},#{h2&0xfff:#x}] @{o:#x}/va{va(o):#x}")

# Broader: MOVS #2 + STRB T1 off 0 (Present byte of trio)
print("\n=== MOVS #2 + STRB [rN,#0] count near SIM (within 0x400 of 0x106a real or 0x18e) ===")
anchors = []
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if r and r[0] in (0x106A, 0x18E, 0x7AB, 0x2C5E):
        anchors.append(o)

hits = []
for o in range(MAIN_OFF, END - 8, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000 or (hw & 0xFF) != 2:
        continue
    rt = (hw >> 8) & 7
    for q in range(o + 2, o + 12, 2):
        h = u16(q)
        if (h & 0xF800) == 0x7000 and (h & 7) == rt and ((h >> 6) & 0x1F) == 0:
            # near an anchor?
            near = any(abs(o - a) < 0x800 for a in anchors)
            if near:
                hits.append((o, (h >> 3) & 7))
            break
print(f"  near-SIM MOVS#2 STRB[rN,#0]: {len(hits)}")
for o, rn in hits[:30]:
    print(f"    @{o:#x} STRB r?,[r{rn},#0]")

# After Pin1Verified=1 at 0x1f04578: does it store #2 to offset 0 of same object (r6)?
print("\n=== Full Pin1Verified success path 0x1f0456c..0x1f045b0 values stored ===")
o = 0x1F0456C
while o < 0x1F045B0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            extra = f" ;BL->{bl_target(o):#x}"
        elif movw(o):
            r = movw(o)
            extra = f" ;MOVW #{r[0]:#x}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS #{hw&0xff}"
        if (hw & 0xF800) == 0x7000:
            extra = f" ;STRB [r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        print(f"  {o:#x}: {hw:04x}{extra}")
        o += 2

# Trace BL 0x1f1458c / 0x1dc53cc — do they set Present=2?
print("\n=== Callees after Pin1Verified: look for MOVS#2 STRB or STRB #0xBF6 ===")
for callee in (0x1F1458C, 0x1DC53CC, 0x1CDFAF0, 0x1F145DE):
    found = []
    for o in range(callee, callee + 0x200, 2):
        hw = u16(o)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for q in range(o + 2, o + 16, 2):
                h = u16(q)
                if (h & 0xF800) == 0x7000:
                    found.append(f"MOVS#2 STRB[r{(h>>3)&7},#{(h>>6)&0x1f}] @{o:#x}")
                if (h & 0xFFF0) == 0xF880:
                    found.append(f"MOVS#2 STRB.W #{u16(q+2)&0xfff:#x} @{o:#x}")
        if (hw & 0xFFF0) == 0xF880 and (u16(o + 2) & 0xFFF) == 0xBF6:
            found.append(f"STRB.W #0xBF6 @{o:#x}")
        if movw(o) and movw(o)[0] == 0x106A:
            found.append(f"LOG 0x106a @{o:#x}")
    print(f"  {callee:#x}: {found[:10]}")

# FCP: search for store of Present-like or call to DetermineSimStatus (0x2c5e)
print("\n=== FCP 0x7ab sites: BL targets + MOVS#2 ===")
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0x7AB:
        continue
    dense = any(
        movw(p) and abs(movw(p)[0] - 0x7AB) <= 1
        for p in range(o + 4, o + 28, 2)
        if movw(p)
    )
    if dense:
        continue
    ctx = None
    for p in range(o - 24, o + 24, 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            if 0x4000 <= imm <= 0x45FF:
                ctx = imm
                break
    if ctx is None:
        continue
    bls = []
    movs2 = False
    for p in range(o - 0x80, o + 0x150, 2):
        b = bl_target(p)
        if b:
            bls.append(b)
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            movs2 = True
    # unique bl targets
    ubls = sorted(set(bls))
    print(f"  0x7ab @{o:#x} ctx={ctx:#x} movs2={movs2} BLs={list(map(hex,ubls))[:8]}")
