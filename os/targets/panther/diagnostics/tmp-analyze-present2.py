#!/usr/bin/env python3
"""Who sets Present=2; link VerifyPin→Present; STRB #0xBF4 near SET_APP; FCP vs Present."""
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


# 1) Context of STRB.W #0xBF4 at 0x1991734 (near SET_APP 0x19916d2)
print("=== SET_APP fn + STRB #0xBF4 @0x1991734 ===")
o = 0x19916C0
while o < 0x19917A0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        if (hw & 0xFFF0) == 0xF880:
            extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            extra = f" ;BL->{bl_target(o):#x}"
        elif movw(o):
            r = movw(o)
            extra = f" ;MOVW r{r[1]},#{r[0]:#x}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB [r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        print(f"  {o:#x}: {hw:04x}{extra}")
        o += 2

# 2) Find all STRB.W to #0xBF6 OR stores that write MOVS #2 into a byte that STATUS reads as Present
# Search: functions that STRB #2 to offset 0 of some object, near SIM strings
# Better approach: find xrefs to log 0x106a args builder — who fills [r5,#0]

# Find who BL's to STATUS function. First find fn start.
print("\n=== STATUS function start + callers ===")
# Look for e92d / b5xx before 0x14fb308 (join point)
for p in range(0x14FB308, 0x14FA800, -2):
    hw = u16(p)
    if (hw & 0xFFF0) == 0xE92D:
        print(f"  PUSH.W @{p:#x}")
        fn = p
        break
    if hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580, 0xB5B8):
        print(f"  PUSH @{p:#x} ={hw:04x}")
        fn = p
        # keep going for outermost
else:
    fn = 0x14FAF00

print(f"  using fn={fn:#x}")

# Find BL to fn (thumb bit)
# Actually BL targets thumb address without +1 in our decoder (file offset)
callers = []
for o in range(MAIN_OFF, END - 4, 2):
    b = bl_target(o)
    if b == fn or (fn < b < fn + 0x20):
        callers.append(o)
print(f"  direct callers of exact fn: {len(callers)}")
for c in callers[:15]:
    print(f"    BL @{c:#x}")

# Also search BL to 0x14fb200 area common entry
for target in (0x14FB000, 0x14FB100, 0x14FB180, 0x14FB1C0, 0x14FB200, 0x14FB218, 0x14FB2CC, 0x14FB308):
    cs = []
    for o in range(MAIN_OFF, END - 4, 2):
        if bl_target(o) == target:
            cs.append(o)
    if cs:
        print(f"  callers of {target:#x}: {len(cs)} e.g. {[hex(x) for x in cs[:5]]}")

# 3) Search MOVS #2; STRB to [rN,#0] within ±0x40 of Pin1Verified OR within functions that also touch offset 20
print("\n=== In Pin1Verified object: any store of #2 to low offsets (0-4) in same fn as STRB#20 ===")
# Function containing 0x1f04576
for p in range(0x1F04576, 0x1F04000, -2):
    if (u16(p) & 0xFFF0) == 0xE92D or u16(p) in (0xB5F0, 0xB570, 0xB5F8):
        pin_fn = p
        break
else:
    pin_fn = 0x1F04400
print(f"  pin1v fn ~{pin_fn:#x}")
for o in range(pin_fn, pin_fn + 0x400, 2):
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3):
        val = hw & 0xFF
        rt = (hw >> 8) & 7
        for q in range(o + 2, o + 12, 2):
            h = u16(q)
            if (h & 0xF800) == 0x7000 and (h & 7) == rt:
                off = (h >> 6) & 0x1F
                rn = (h >> 3) & 7
                print(f"  MOVS#{val} STRB [r{rn},#{off}] @{o:#x}")
            if (h & 0xFFF0) == 0xF880 and ((u16(q + 2) >> 12) & 0xF) == rt:
                print(f"  MOVS#{val} STRB.W [r{h&0xf},#{u16(q+2)&0xfff:#x}] @{o:#x}")

# 4) Re-check: maybe READY CMP #2 compares Pin1Verified+MePerVerified packed or Present is enum
# Look for MOVS #2 stores to offset matching Present source used by STATUS
# Search string "Present" near USIM that isn't RRC
print("\n=== USIM Present strings ===")
for n in [b"SimPresent", b"usimPresent", b"UsimPresent", b"isPresent", b"IsSimPresent",
          b"SIM Present", b"sim present", b"Card Present", b"cardPresent",
          b"Present, Pin1", b"MePerVerified"]:
    j = img.find(n)
    if j >= 0:
        s0 = j
        while s0 > 0 and 32 <= img[s0 - 1] < 127:
            s0 -= 1
        s1 = j
        while s1 < len(img) and 32 <= img[s1] < 127:
            s1 += 1
        print(f"  {s0:#x}: {img[s0:s1].decode()[:120]}")

# 5) Critical: does STATUS compare +0xBF6 AFTER also writing other values?
# Look at path to READY - was Present rewritten?
# Between 0x14fb380 and 0x14fb5bc, any other STRB to #0xBF6? Already know only one.
# So Present source [r5,#0] must already be 2 before STATUS runs for READY.

# Find writers that store #2 into a byte later read as Present for log 0x106a
# Heuristic: search MOVS #2; STRB within 0x200 bytes of code that also does MOVS #1; STRB #20 (Pin1Verified pattern)
print("\n=== Functions that both set Pin1Verified (#20=1) and store #2 somewhere ===")
# Already know Pin1Verified sites - scan whole those functions for MOVS#2 STRB
for pin_fn_start, pin_fn_end in [(0x1F04400, 0x1F04600), (0x1F04678, 0x1F04710)]:
    print(f"  range {pin_fn_start:#x}-{pin_fn_end:#x}")
    for o in range(pin_fn_start, pin_fn_end, 2):
        hw = u16(o)
        if (hw & 0xFF00) == 0x2000:
            print(f"    MOVS r{(hw>>8)&7},#{hw&0xff} @{o:#x}")

# 6) Alternative theory: MLA decode wrong — FB16 8000 might be UBFX/SBFX or ADD
# Check: could f890 0bf6 be LDRB from [r0, #0xBF6] where r0 is NOT the Present object
# but a status enum object, and the STRB of Present is to a *different* indexed object
# because r6 vs r5 strides differ!
#
# STRB Present: MLA with r6 (stride A)
# CMP#1 PUK: MLA with r5 (stride B)  
# READY: MLA with r4 (stride C)
#
# If strides differ, +0xBF6 on different arrays! Present write ≠ READY read object!
# Then who writes READY's +0xBF6?

print("\n=== What sets r4/r5/r6 before each MLA in STATUS ===")
# dump from 0x14fb300 with focus on MOV to r4/r5/r6
o = 0x14FB300
while o < 0x14FB5D0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        if (hw & 0xFF00) == 0xFB00 and (hw2 & 0xF0) == 0:
            rn, ra, rd, rm = hw & 0xF, (hw2 >> 12) & 0xF, (hw2 >> 8) & 0xF, hw2 & 0xF
            kind = "MUL" if ra == 0xF else "MLA"
            print(f"  {o:#x}: {kind} r{rd},r{rn},r{rm}" + (f",r{ra}" if ra != 0xF else ""))
        o += 4
        continue
    # MOV rD, rM encoding 4600-46FF
    if (hw & 0xFF00) == 0x4600:
        rd = ((hw >> 3) & 1) << 3 | (hw & 7)  # rough
        # Actually MOV low: 0100 0110 00 mm mm dd d
        rm = (hw >> 3) & 0xF
        rd = ((hw & 0x80) >> 4) | (hw & 7)  # wrong
        # Standard: bits [7]=D, [6:3]=Rm, [2:0]=Rd
        rd = ((hw >> 7) & 1) << 3 | (hw & 7)
        rm = (hw >> 3) & 0xF
        if rd in (4, 5, 6) or rm in (4, 5, 6):
            print(f"  {o:#x}: MOV r{rd},r{rm}")
    if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) in (4, 5, 6):
        print(f"  {o:#x}: MOVS r{(hw>>8)&7},#{hw&0xff}")
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        o += 4
    else:
        o += 2
