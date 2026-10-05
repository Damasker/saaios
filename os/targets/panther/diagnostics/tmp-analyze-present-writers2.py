#!/usr/bin/env python3
"""Present writers filtered; SIM-wrap case msg; SIT INIT/STACK opcodes."""
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
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
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
            elif (hw & 0xF800) == 0x4800:
                lit = ((o + 4) & ~3) + (hw & 0xFF) * 4
                extra = f" ;LDR r{(hw>>8)&7},[PC]->{hex(lit)}={hex(u32(lit)) if lit+4<=len(img) else '?'}"
            elif (hw & 0xF000) == 0xD000:
                extra = f" ;Bcond"
            elif (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm & 0x400:
                    imm -= 0x800
                extra = f" ;B->{hex(o+4+imm*2)}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


# ---- Present object: STATUS reads [r5,#0]; FN_A writes [r8,#0] and [r8,#1]
# Find all STRB.W to [rn,#0] where same rn also has STRB/LDRB #1 or #8 nearby (Present trio)
print("=== Present-object pattern stores in 0x14f0000..0x1508000 ===")
writers = []
for o in range(0x14F0000, 0x1508000, 2):
    hw, hw2 = u16(o), u16(o + 2)
    # STRB.W rt,[rn,#0]
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
        rn = hw & 0xF
        rt = (hw2 >> 12) & 0xF
        # find value
        val = None
        for b in range(2, 32, 2):
            h = u16(o - b)
            if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt and (h & 0xFF) <= 3:
                val = h & 0xFF
                break
        if val is None:
            continue
        # confirm Present-like: access #1 or #8 on same rn in ±0x40
        related = False
        for p in range(max(0x14F0000, o - 0x60), min(0x1508000, o + 0x60), 2):
            h, h2 = u16(p), u16(p + 2)
            if (h & 0xFFF0) in (0xF880, 0xF890) and (h & 0xF) == rn and (h2 & 0xFFF) in (1, 8):
                related = True
                break
            # STRB T1 imm 1 or 8
            if (h & 0xF800) in (0x7000, 0x7800) and ((h >> 3) & 7) == rn and ((h >> 6) & 0x1F) in (1, 8):
                related = True
                break
        if related:
            logs = []
            for p in range(max(0x14F0000, o - 0x50), o + 0x30, 2):
                r = movw(p)
                if r and r[0] < 0x2000:
                    logs.append(hex(r[0]))
            writers.append((o, val, rn, rt, logs[:5]))

print(f"pattern writers: {len(writers)}")
for o, val, rn, rt, logs in writers:
    print(f"  Present={val} STRB.W [r{rn},#0] @{hex(o)} logs={logs}")

# Also T1 STRB [rn,#0] with related #1/#8
print("\n=== T1 STRB #0 with related #1/#8 ===")
for o in range(0x14F0000, 0x1508000, 2):
    hw = u16(o)
    if (hw & 0xF800) != 0x7000 or ((hw >> 6) & 0x1F) != 0:
        continue
    rt, rn = hw & 7, (hw >> 3) & 7
    val = None
    for b in range(2, 24, 2):
        h = u16(o - b)
        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt and (h & 0xFF) <= 3:
            val = h & 0xFF
            break
    if val is None:
        continue
    related = False
    for p in range(max(0x14F0000, o - 0x50), min(0x1508000, o + 0x50), 2):
        h, h2 = u16(p), u16(p + 2)
        if (h & 0xFFF0) in (0xF880, 0xF890) and (h & 0xF) == rn and (h2 & 0xFFF) in (1, 8):
            related = True
            break
        if (h & 0xF800) in (0x7000, 0x7800) and ((h >> 3) & 7) == rn and ((h >> 6) & 0x1F) in (1, 8):
            related = True
            break
    if related:
        print(f"  Present={val} STRB [r{rn},#0] @{hex(o)}")

# ---- SIM-wrap detailed ----
print("\n=== WRAP_A 0x14c380e (for compare) ===")
print(dump(0x14C3800, 0x14C38A0))

print("\n=== Case body around 0x14c3986 ===")
print(dump(0x14C3930, 0x14C3A00))

print("\n=== WRAP_SIM 0x14f6d02 full ===")
print(dump(0x14F6D02, 0x14F6DC0))

# What calls into the switch? Find function with MMC string / log near WRAP_A
print("\n=== Logs near WRAP_A / 0x14c3986 switch (±0x200) ===")
for o in range(0x14C3600, 0x14C3C00, 2):
    r = movw(o)
    if not r:
        continue
    for q in range(o - 16, o + 20, 2):
        t = movt(q)
        if t and t[1] == r[1] and 0x4000 <= t[0] <= 0x45FF:
            print(f"  {hex(o)}: imm={hex(r[0])} ctx={hex(t[0])}")
            break

# Map log imm to string if A[ format with known packing - search 0x1068 near Present log
print("\n=== Strings near SIM STATUS / 0x1068 candidates ===")
# Find all strings containing "Present" and "Pin1" or "SIM STATUS"
for off, s in find_str(b"SIM STATUS"):
    print(f"  @{hex(off)}: {s[:120]}")
for off, s in find_str(b"Present:%d, Pin1"):
    print(f"  @{hex(off)}: {s[:120]}")

# Switch dispatch: look at 0x14c3700 for TBB / table branch
print("\n=== Switch preamble 0x14c3700..0x14c3830 ===")
print(dump(0x14C3700, 0x14C3830))

# ---- arg0=[obj,#8] in WRAP_SIM path ----
print("\n=== FN_A arg path: who sets r0 before BL FN_A at 0x14f6d7a ===")
print(dump(0x14F6D40, 0x14F6D90))

# WRAP_A sets arg from LDRB [msg+0]
print("\n=== WRAP_A arg to FN_A ===")
print(dump(0x14C3840, 0x14C3880))

# ---- SIT / factory: INIT and STACK_SERVICES ----
print("\n=== SIT/factory strings for INIT / STACK_SERVICES ===")
for needle in (
    b"SIM_INIT_REQ",
    b"START_STACK_SERVICES",
    b"sitInformSimInit",
    b"InformSimInit",
    b"BuildSimInit",
    b"SimInit",
    b"STACK_SERVICES_REQ",
    b"sitTxSimInit",
    b"sitSendSimInit",
    b"NS_SIM_INIT",
    b"USIM_INIT",
    b"sitRxSimStart",
    b"SimStartStack",
):
    hits = find_str(needle)
    for off, s in hits[:4]:
        print(f"  @{hex(off)}: {s[:120]}")

# SIT builders often logged as A[SIT_*_SIM] Tx ...
print("\n=== A[SIT_*_SIM] Tx/Rx mentioning Init/Start/Stack ===")
for off, s in find_str(b"A[SIT_0_SIM]"):
    if any(k in s for k in ("Init", "START", "Stack", "INIT", "Start")):
        print(f"  @{hex(off)}: {s[:120]}")
for off, s in find_str(b"A[SIT_1_SIM]"):
    if any(k in s for k in ("Init", "START", "Stack", "INIT", "Start")):
        print(f"  @{hex(off)}: {s[:120]}")

# sitInformSimInit body — find via format string VA... A-logs use id packing.
# Search for nearby SIT opcode constants: often movw #0x02xx before sit send
print("\n=== Around sitInformSimInit string — search code refs via litpool word ===")
for off, s in find_str(b"sitInformSimInit()"):
    v = va(off)
    nb = struct.pack("<I", v)
    i = 0
    pools = []
    while True:
        j = img.find(nb, i)
        if j < 0:
            break
        pools.append(j)
        i = j + 1
    print(f"{s} VA={hex(v)} pools={list(map(hex, pools[:5]))}")

# Message name SIM_INIT_REQ — find handler via table index
# Pool at 0x10fb07c from earlier; dump neighboring msg ids as offsets into name table
print("\n=== Name table around INIT_REQ / START_STACK ===")
for label, needle in (
    ("INIT_REQ", b"USIM <== SIM_INIT_REQ"),
    ("START_STACK", b"USIM <== SIM_START_STACK_SERVICES_REQ"),
    ("START_IND", b"USIM ==> SIM_START_IND"),
):
    off = find_str(needle)[0][0]
    v = va(off)
    nb = struct.pack("<I", v)
    j = img.find(nb)
    print(f"{label} name@{hex(off)} pool@{hex(j) if j>=0 else None}")
    if j >= 0:
        # table of ptrs — find start by scanning back while ptrs look like name VAs
        k = j
        while k > 0x100000:
            w = u32(k - 4)
            if not (0x41020000 <= w <= 0x41080000):
                break
            k -= 4
        idx = (j - k) // 4
        print(f"  table_start~{hex(k)} index~{idx}")
        for i in range(max(0, idx - 2), idx + 3):
            w = u32(k + i * 4)
            fo = MAIN_OFF + (w - VA_BASE)
            if 0 <= fo < len(img):
                b = fo
                while b < fo + 60 and 32 <= img[b] < 127:
                    b += 1
                print(f"    [{i}] {img[fo:b].decode('ascii','replace')}")

print("\nDONE")
