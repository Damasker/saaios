#!/usr/bin/env python3
"""Present enum all writers 0-3; SIM-wrap 0x14c3986; INIT/STACK SIT."""
import struct
from pathlib import Path
from collections import defaultdict

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

FN_A, FN_B = 0x14F692C, 0x14F9108
WRAP_SIM = 0x14F6D02
CALLER_WRAP = 0x14C3986


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


def find_str(needle: bytes):
    out = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        out.append((a, img[a:b].decode("ascii", "replace")))
        i = j + 1
    return out


def dump(start, end):
    o = start
    lines = []
    while o < end and o < END - 4:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t = movw(o), movt(o)
            bt = bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                tag = ""
                if bt in (FN_A, FN_B, WRAP_SIM):
                    tag = " *"
                extra = f" ;BL->{hex(bt)}{tag}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                extra = f" ;MOV r{hw&7},r{(hw>>3)&7}"
            elif (hw & 0xF800) == 0x6800:
                extra = f" ;LDR r{hw&7},[r{(hw>>3)&7},#{((hw>>6)&0x1f)*4}]"
            elif (hw & 0xF800) == 0x6000:
                extra = f" ;STR r{hw&7},[r{(hw>>3)&7},#{((hw>>6)&0x1f)*4}]"
            elif (hw & 0xF800) == 0x4800:
                lit = ((o + 4) & ~3) + (hw & 0xFF) * 4
                extra = f" ;LDR r{(hw>>8)&7},[PC] ->{hex(lit)} ={hex(u32(lit)) if lit+4<=len(img) else '?'}"
            elif hw == 0x4770:
                extra = " ;BX LR"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


# ========== 1) Disasm FN_A / FN_B Present stores ==========
print("=== FN_A 0x14f692c (full) ===")
print(dump(FN_A, FN_A + 0x120))

print("\n=== FN_B 0x14f9108 Present region ===")
# find Present=2 site around 0x14f9578
print(dump(0x14F9500, 0x14F9600))
print("\n--- FN_B start ---")
print(dump(FN_B, FN_B + 0x80))

# ========== 2) Find Present object pointer used by FN_A ==========
# From dump we'll see LDR of global. Also search SIM module 0x14f0000-0x1500000
# for MOVS r?,#0/1/2/3 + STRB to [rN,#0] where rN is Present ptr.

print("\n=== SIM module 0x14f0000..0x1502000: MOVS#0-3 + STRB [rn,#0] ===")
# Also STRB.W with imm 0
sites = []
for o in range(0x14F0000, 0x1502000, 2):
    hw = u16(o)
    # MOVS rd,#imm8 with imm in 0..3
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) <= 3:
        rd = (hw >> 8) & 7
        val = hw & 0xFF
        for p in range(o + 2, min(o + 24, 0x1502000), 2):
            h = u16(p)
            # STRB rt,[rn,#0] T1: imm5=0
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0 and (h & 7) == rd:
                rn = (h >> 3) & 7
                sites.append((o, p, val, rd, rn, "STRB#0"))
                break
            # STRB.W rt,[rn,#0]
            if (h & 0xFFF0) == 0xF880 and (u16(p + 2) & 0xFFF) == 0 and ((u16(p + 2) >> 12) & 0xF) == rd:
                rn = h & 0xF
                sites.append((o, p, val, rd, rn, "STRB.W#0"))
                break
            # IT EQ / conditional — keep looking
            if bl_target(p):
                break

# Dedup and show with nearby log ids
print(f"count={len(sites)}")
for mov_o, str_o, val, rd, rn, kind in sites:
    logs = []
    for p in range(max(0x14F0000, mov_o - 0x40), mov_o + 0x20, 2):
        r = movw(p)
        if r and r[0] < 0x2000:
            logs.append(hex(r[0]))
    print(f"  Present?={val} {kind} mov@{hex(mov_o)} str@{hex(str_o)} r{rd}->[r{rn},#0] logs~{logs[:6]}")

# ========== 3) Also search for STRB to Present via absolute VA ==========
# STATUS loads Present from somewhere into [r5,#0] before log — find LDR of Present global
print("\n=== STATUS Present load path @0x14fb308..0x14fb390 ===")
print(dump(0x14FB300, 0x14FB390))

# ========== 4) SIM-wrap caller 0x14c3986 ==========
print("\n=== SIM-wrap caller region 0x14c3900..0x14c3a40 ===")
print(dump(0x14C3900, 0x14C3A40))

print("\n=== WRAP_SIM fn 0x14f6d02 ===")
print(dump(WRAP_SIM, WRAP_SIM + 0xA0))

# Find switch / message id near 0x14c3986 — look back for case dispatch
print("\n=== Walk back from 0x14c3986 for switch/log ===")
print(dump(0x14C3700, 0x14C3990))

# Log 0x1068 string
print("\n=== log 0x1068 / nearby STATUS strings ===")
for n in (
    b"SIM STATUS update",
    b"Present:%d",
    b"0x1068",
):
    pass
# find format strings that might map to 0x1068 by proximity to 0x106a string
for off, s in find_str(b"SIM STATUS update"):
    print(f"  @{hex(off)}: {s[:100]}")
for off, s in find_str(b"Present:%d"):
    print(f"  @{hex(off)}: {s[:100]}")

# Search A[ strings around "Present" in USIM/SIM
print("\n=== Present-related A/log strings ===")
for needle in (
    b"Present:%d, Pin1Verified",
    b"SetPresent",
    b"m_Present",
    b"SimPresentStatus",
    b"SIM present",
    b"Present =",
    b"present status",
    b"USIM_PRESENT",
    b"Card Present",
):
    for off, s in find_str(needle):
        print(f"  @{hex(off)}: {s[:110]}")

# Find which switch case contains 0x14c3986 — look for log MOVW in function
print("\n=== Logs in fn containing 0x14c3986 ===")
# find fn start
fs = None
for p in range(0x14C3986, 0x14C3986 - 0x800, -2):
    if (u16(p) & 0xFFF0) == 0xE92D:
        fs = p
        break
print(f"fn_start={hex(fs) if fs else None}")
if fs:
    for p in range(fs, min(fs + 0x600, END - 4), 2):
        r = movw(p)
        if not r:
            continue
        # paired movt for ctx
        for q in range(p - 16, p + 20, 2):
            t = movt(q)
            if t and t[1] == r[1] and 0x4000 <= t[0] <= 0x45FF:
                if r[0] < 0x3000 or r[0] in (0x1068, 0x106A, 0x106B, 0x2C5E):
                    print(f"  log? {hex(r[0])} ctx={hex(t[0])} @{hex(p)}")
                break

# ========== 5) Who calls 0x14c3986? ==========
print("\n=== Callers of 0x14c3986 / WRAP_SIM ===")
c3986 = []
cwrap = []
for o in range(MAIN_OFF, END - 4, 2):
    bt = bl_target(o)
    if bt == 0x14C3986:
        c3986.append(o)
    if bt == WRAP_SIM:
        cwrap.append(o)
print(f"BL 0x14c3986: {list(map(hex, c3986))}")
print(f"BL WRAP_SIM: {list(map(hex, cwrap))}")
# Maybe 0x14c3986 is not a function start but a case label — find BLs to nearby
# Search for BL to addresses in 0x14c3980..0x14c39c0
near = defaultdict(list)
for o in range(MAIN_OFF, END - 4, 2):
    bt = bl_target(o)
    if bt and 0x14C3900 <= bt <= 0x14C3A00:
        near[bt].append(o)
print("BLs into 0x14c3900..0x14c3a00:", {hex(k): list(map(hex, v[:5])) for k, v in near.items()})

print("\nDONE part1")
