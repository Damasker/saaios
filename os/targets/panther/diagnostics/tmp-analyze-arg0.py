#!/usr/bin/env python3
"""Map arg0 byte meaning; callers of wrappers; FCP/VerifyPin/init links."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A = 0x14F692C
WRAP_A = 0x14C380E  # LDRB [r4,#0/#1]; BL FN_A
HELPER = 0x14F6900  # small LDRB return
WRAP_H = 0x14C37AC  # BL HELPER
FN_B = 0x14F9108


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


def dump(start, end):
    o = start
    lines = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = ""
                if r[0] in (0x7AB, 0x18E, 0x106A, 0x1068, 0xC6B, 0x2C5E, 0xBDA):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                tgt = bl_target(o)
                tag = ""
                if tgt == FN_A:
                    tag = " <<FN_A"
                elif tgt == WRAP_A:
                    tag = " <<WRAP_A"
                elif tgt in (0x1E0C174, 0x1E0C2FE):
                    tag = " <<FCP?"
                extra = f" ;BL->{tgt:#x}{tag}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            elif (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            lines.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            extra = f" ;MOV r{rd},r{rm}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF000) == 0xD000 and (hw & 0x0F00) < 0x0E00:
            imm8 = hw & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            extra = f" ;Bcond->{o+4+(imm8<<1):#x}"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


def find_callers(target):
    return [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == target]


# Callers of WRAP_A and WRAP_H and FN_B
for name, tgt in [("WRAP_A", WRAP_A), ("WRAP_H", WRAP_H), ("HELPER", HELPER), ("FN_B", FN_B)]:
    cs = find_callers(tgt)
    print(f"=== {name} {tgt:#x} callers ({len(cs)}) ===")
    for c in cs[:20]:
        print(f"  @{c:#x}/va{va(c):#x}")
        # preceding MOVS / LDRB into r0
        for b in range(2, 48, 2):
            hw = u16(c - b)
            if (hw & 0xFF00) == 0x2000:
                print(f"    MOVS r{(hw>>8)&7},#{hw&0xff}")
            if (hw & 0xF800) == 0x7800 and (hw & 7) == 0:
                print(f"    LDRB r0,[r{(hw>>3)&7},#{(hw>>6)&0x1f}]")
            if (hw & 0xFF00) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                if rd == 0:
                    print(f"    MOV r0,r{rm}")
            r = movw(c - b)
            if r and r[0] in (0x7AB, 0x18E, 0xC6B, 0x2C5E, 0x106A, 0x1068):
                print(f"    MOVW LOG_{r[0]:#x}")

# Dump WRAP_A callers fully
print("\n=== WRAP_A caller dumps ===")
for c in find_callers(WRAP_A):
    print(f"\n---- @{c:#x} ----")
    print(dump(c - 0x80, c + 0x10))

# Second FN_A caller 0x14f6d7a — what is [r5,#8]?
print("\n=== FN_A internal caller 0x14f6d7a wider context ===")
print(dump(0x14F6C80, 0x14F6D90))

# Who stores byte value 3 into structs that WRAP_A reads?
# Search: MOVS #3; STRB [rN,#0] near SIM / near callers
print("\n=== MOVS#3 STRB[rN,#0] near WRAP_A callers / FN_A / FCP / pin1v ===")
anchors = find_callers(WRAP_A) + find_callers(FN_A) + [0x1E0C2FE, 0x1F04576, 0x14F6A14]
for o in range(MAIN_OFF, END - 8, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000 or (hw & 0xFF) != 3:
        continue
    rt = (hw >> 8) & 7
    for q in range(o + 2, o + 12, 2):
        h = u16(q)
        if (h & 0xF800) == 0x7000 and (h & 7) == rt and ((h >> 6) & 0x1F) == 0:
            if any(abs(o - a) < 0x1000 for a in anchors):
                print(f"  MOVS#3 STRB[r{(h>>3)&7},#0] @{o:#x} near-anchor")
            break
        if (h & 0xFFF0) == 0xF880 and ((u16(q + 2) >> 12) & 0xF) == rt and (u16(q + 2) & 0xFFF) == 0:
            if any(abs(o - a) < 0x1000 for a in anchors):
                print(f"  MOVS#3 STRB.W[r{h&0xf},#0] @{o:#x}")
            break

# Does FCP or sitSetPin1 BL WRAP_A / FN_A / WRAP_H?
print("\n=== From FCP/0x18e/sitSetPin1 regions: BL to our targets? ===")
targets = {FN_A, WRAP_A, WRAP_H, HELPER, FN_B, 0x14F6A14}
# scan functions around known sites
regions = [
    ("FCP", 0x1E0C000, 0x1E0C800),
    ("Pin1V", 0x1F04400, 0x1F04800),
    ("sitSetPin1_scan", 0x1991600, 0x1991A00),
]
# also find real 0xc6b and scan ±0x400
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0xC6B:
        continue
    dense = any(movw(p) and abs(movw(p)[0] - 0xC6B) <= 1 for p in range(o + 4, o + 24, 2) if movw(p))
    if dense:
        continue
    for p in range(o - 0x200, o + 0x400, 2):
        b = bl_target(p)
        if b in targets or (b and 0x14C37AC <= b <= 0x14C3900) or (b and 0x14F6900 <= b <= 0x14F6B00):
            print(f"  sitSetPin1 vicinity @{o:#x}: BL @{p:#x}->{b:#x}")

for name, start, end in regions:
    hits = []
    for o in range(start, end, 2):
        b = bl_target(o)
        if b in targets or (b and (0x14C3700 <= b <= 0x14C3900 or 0x14F6900 <= b <= 0x14F6D00 or 0x14F9100 <= b <= 0x14F9600)):
            hits.append((o, b))
    print(f"  {name}: {hits[:12]}")

# Log 0x1068 — find string via broader scan
print("\n=== Search strings near Present builder ===")
for n in [
    b"Present",
    b"MePerVerified",
    b"SimStatus update",
    b"SIM status",
    b"AppType",
    b"app_type",
    b"USIM init",
    b"InitComplete",
    b"init complete",
    b"ATR",
    b"Select ADF",
]:
    # only in log string area
    pass
# find 0x1068 by scanning all shannon hdrs with that id near known
count = 0
for off in range(0x4C00000, 0x4E00000, 4):
    w = struct.unpack_from("<I", img, off)[0]
    if (w & 0xFF) == 0x44 and ((w >> 8) & 0xFFFF) == 0x1068:
        s = off + 8
        if 32 <= img[s] < 127:
            end = img.find(b"\0", s)
            print(f"  0x1068 @{s:#x}: {img[s:end].decode()[:140]}")
            count += 1
            if count >= 3:
                break

# Enum: CMP #3 in FN_A — also CMP other values for Present?
print("\n=== FN_A Present assignment ladder (all STRB [r8,#0]) ===")
print(dump(0x14F69D0, 0x14F6B20))
