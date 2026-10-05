#!/usr/bin/env python3
"""RO: callers of Present=2 builders; arg0 meaning; FCP/VerifyPin/ATR links."""
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


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
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
            t = movt(o)
            if r:
                note = ""
                if r[0] in (
                    0x7AB,
                    0x18E,
                    0x106A,
                    0x1068,
                    0xC6B,
                    0xC5B,
                    0xBDA,
                    0x2C5E,
                ):
                    note = f" ;LOG_{r[0]:#x}"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif t:
                extra = f" ;MOVT r{t[1]},#{t[0]:#x}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
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
        elif (hw & 0xF800) == 0xE000:
            imm11 = hw & 0x7FF
            if imm11 >= 0x400:
                imm11 -= 0x800
            extra = f" ;B->{o+4+(imm11<<1):#x}"
        lines.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(lines)


# --- Find fn entries ---
FN_A = 0x14F692C  # Present=2 builder A (arg0→r9)
# Find fn containing 0x14f9578
fn_b = None
for p in range(0x14F9578, 0x14F9000, -2):
    hw = u16(p)
    if (hw & 0xFFF0) == 0xE92D or hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580):
        fn_b = p
        break
print(f"FN_A={FN_A:#x} va={va(FN_A):#x}")
print(f"FN_B guess={fn_b:#x}" if fn_b else "FN_B guess=None")

# --- All callers of FN_A and FN_B ---
print("\n=== Callers of FN_A 0x14f692c ===")
callers_a = []
for o in range(MAIN_OFF, END - 4, 2):
    if bl_target(o) == FN_A:
        callers_a.append(o)
print(f"  count={len(callers_a)}: {[hex(c) for c in callers_a]}")

print("\n=== Callers of FN_B ===")
callers_b = []
if fn_b:
    for o in range(MAIN_OFF, END - 4, 2):
        if bl_target(o) == fn_b:
            callers_b.append(o)
    print(f"  fn_b={fn_b:#x} count={len(callers_b)}: {[hex(c) for c in callers_b[:20]]}")

# Also search BL to nearby entry points around 0x14f9400
for target in range(0x14F9100, 0x14F9500, 2):
    hw = u16(target)
    if not ((hw & 0xFFF0) == 0xE92D or hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0)):
        continue
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == target]
    if cs:
        print(f"  entry {target:#x} callers={len(cs)} {[hex(c) for c in cs[:8]]}")

# --- Context of each FN_A caller: what is in r0/r4 before BL ---
print("\n=== FN_A caller contexts (arg0 setup) ===")
for c in callers_a:
    print(f"\n---- caller @{c:#x}/va{va(c):#x} ----")
    print(dump(c - 0x60, c + 0x20))

# --- Walk up from 0x14c37f2: what function, what sets r4 ---
print("\n=== Function containing 0x14c37f2 ===")
fn_caller = None
for p in range(0x14C37F2, 0x14C3000, -2):
    hw = u16(p)
    if (hw & 0xFFF0) == 0xE92D:
        fn_caller = p
        print(f"  PUSH.W @{p:#x}")
        break
    if hw in (0xB5F0, 0xB5F8, 0xB570, 0xB5B0, 0xB580, 0xB5B8):
        fn_caller = p
        print(f"  PUSH @{p:#x}={hw:04x}")
# dump from fn start to BL
if fn_caller:
    print(f"\n=== dump {fn_caller:#x} .. 0x14c3810 (r4 origin) ===")
    print(dump(fn_caller, 0x14C3810))

# Callers of fn containing 0x14c37f2
if fn_caller:
    cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == fn_caller]
    print(f"\n=== Callers of wrapper {fn_caller:#x}: {len(cs)} ===")
    for c in cs[:15]:
        print(f"  @{c:#x}")
        # show MOVS imm before BL
        for b in range(2, 40, 2):
            hw = u16(c - b)
            if (hw & 0xFF00) == 0x2000:
                print(f"    MOVS r{(hw>>8)&7},#{hw&0xff} @{c-b:#x}")
            r = movw(c - b)
            if r and r[0] in (0x7AB, 0x18E, 0xC6B, 0x2C5E, 0x106A, 3, 1, 2):
                print(f"    MOVW r{r[1]},#{r[0]:#x} @{c-b:#x}")
