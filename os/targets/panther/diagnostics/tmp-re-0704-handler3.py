#!/usr/bin/env python3
"""Resolve SIT 0x0704 registered handler; dump preconditions → error 2."""
from __future__ import annotations
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/"
    "diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
START_NET = 0x18E8028


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def bcond_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
    s = (hw >> 10) & 1
    cond = (hw >> 6) & 0xF
    imm6 = hw & 0x3F
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 20) | (i1 << 19) | (i2 << 18) | (imm6 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32, cond


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


def dump(start, end, lab):
    print(f"\n=== {lab} [{hex(start)}..{hex(end)}] ===")
    o = start
    while o < end:
        hw = u16(o)
        extra = ""
        mw, mt, b = movw(o), movt(o), bl_target(o)
        bc = bcond_w(o)
        if mw:
            extra = f" MOVW r{mw[1]},#{hex(mw[0])}"
        if mt:
            extra = f" MOVT r{mt[1]},#{hex(mt[0])}"
        if b is not None:
            tag = ""
            if b == GET_APP:
                tag = " GET_APP"
            elif abs(b - START_NET) < 8:
                tag = " START_NET"
            extra = f" BL->{hex(b)}{tag}"
        if bc:
            extra += f" Bcond.W->{hex(bc[0])} c={bc[1]}"
        if (hw & 0xFF00) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            hw2 = u16(o + 2)
            extra += f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            # BLX reg
            if (hw & 0xFF87) == 0x4780:
                extra += f" BLX r{(hw>>3)&0xf}"
            if (hw & 0xFF87) == 0x4700:
                extra += f" BX r{(hw>>3)&0xf}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# --- Find opcode-only MOVW #0x704 (no immediate MOVT same rd within +8) ---
print("=== opcode-like MOVW #0x0704 (no MOVT same rd in +8) ===")
op_sites = []
o = 0
while o + 8 < len(IMG):
    mw = movw(o)
    if mw and mw[0] == 0x704:
        mt = movt(o + 4)
        if mt and mt[1] == mw[1]:
            o += 2
            continue
        # also skip if MOVT within +8
        skip = False
        for j in (4, 6, 8):
            mt2 = movt(o + j)
            if mt2 and mt2[1] == mw[1]:
                skip = True
                break
        if not skip:
            op_sites.append((o, mw[1]))
    o += 2
print(f"count={len(op_sites)}")
for o, rd in op_sites[:50]:
    near = []
    for j in range(max(0, o - 0x30), min(len(IMG) - 4, o + 0x40), 2):
        mw = movw(j)
        if mw and 0x700 <= mw[0] <= 0x715:
            near.append(f"{hex(j)}:#{hex(mw[0])}")
        if (u16(j) & 0xFF00) == 0x2000 and (u16(j) & 0xFF) == 2:
            near.append(f"{hex(j)}:MOVS#2")
        b = bl_target(j)
        if b == GET_APP:
            near.append(f"{hex(j)}:GET_APP")
        if b == START_NET:
            near.append(f"{hex(j)}:START_NET")
    print(f"  {hex(o)} r{rd} {near[:12]}")

# --- Deep dump registration cluster at 0x11414d0 / 0x1150ae0 / 0x11fad40 ---
for base in (0x11414D0, 0x1150AE0, 0x11FAD40, 0x1351480, 0x17A9A70):
    dump(base, base + 0x100, f"cluster_{hex(base)}")

# --- Resolve handler: at 0x114166e pattern MOVW#704; mov r1,#n; blx r3 ---
# Look backward for where r2/r3 got function pointer
print("\n=== backtrace reg site 0x114166e for handler pointer ===")
dump(0x1141640, 0x11416A0, "reg704_focus")

# Search literal pools / tables containing 0x00000704 followed by function ptr
print("\n=== data tables: u16 0x0704 then u32 looking like Thumb code ptr ===")
hits = 0
o = 0
while o + 8 < len(IMG) and hits < 40:
    if u16(o) == 0x0704 and (o % 2 == 0):
        # aligned table entries often 8 or 12 bytes
        for stride in (4, 8, 12, 16):
            if o + stride + 4 > len(IMG):
                continue
            ptr = u32(o + 2) if False else u32(o + 4)  # often id then pad then ptr
            # try several layouts
        layouts = [
            ("id+ptr", o, u32(o + 4) if o + 8 <= len(IMG) else 0),
            ("id+u16+ptr", o, u32(o + 4) if o + 8 <= len(IMG) else 0),
            ("u32id+ptr", o - 2 if o >= 2 and u32(o - 2) & 0xFFFF == 0x704 else -1, 0),
        ]
        # simpler: if next 4 bytes look like code address in MAIN ranges
        for off in (2, 4, 6, 8):
            if o + off + 4 > len(IMG):
                continue
            ptr = u32(o + off) & ~1
            if 0x1000000 <= ptr <= 0x2800000:
                # verify looks like function prolog
                hw = u16(ptr)
                if hw in (0xB500, 0xB510, 0xB570, 0xB5F0, 0xB5F8, 0xB580, 0xE92D) or (hw & 0xFF00) == 0xB500:
                    print(f"  table@{hex(o)} +{off} -> handler {hex(ptr)} prolog={hw:04x}")
                    hits += 1
                    break
    o += 2

# --- Alternate: find BL START_NET from functions that also reference opcode 0x704 via literal ---
print("\n=== functions that both touch opcode 704 and START_NET ===")
# Scan for MOVW#704 opcode-like, then search forward 0x400 for BL START_NET
linked = []
for o, rd in op_sites:
    for j in range(o, min(o + 0x400, len(IMG) - 4), 2):
        if bl_target(j) == START_NET:
            linked.append((o, j))
            break
        # also GET_APP CMP 1/4/5 pattern
print(f"linked opcode704→START_NET: {linked[:20]}")

# Search op_sites for GET_APP within ±0x200
for o, rd in op_sites:
    apps = []
    for j in range(max(0, o - 0x200), min(o + 0x200, len(IMG) - 4), 2):
        if bl_target(j) == GET_APP:
            cmps = []
            for k in range(j + 4, j + 0x30, 2):
                hw = u16(k)
                if (hw & 0xFF00) == 0x2800:
                    cmps.append(hw & 0xFF)
            apps.append((j, cmps))
    if apps:
        print(f"  op@{hex(o)} GET_APP near: {[(hex(a), c) for a, c in apps[:4]]}")

# --- Decode fail path at START_NET more carefully ---
# After CMP#5 BNE.W — dump BOTH fallthrough and branch target with log string search
bc = bcond_w(0x18E8330)
print(f"\nBNE after CMP#5: {bc}")
if bc:
    dump(bc[0], bc[0] + 0x80, "bne_target")
    # search backward from bne target for MOVS #2 / error complete helper
    dump(bc[0] - 0x40, bc[0] + 0x40, "bne_target_back")

# Fallthrough after CMP#5 success
dump(0x18E8334, 0x18E8480, "cmp5_success_continue")

# Find where fail path sets error — search MOVS #2 in START_NET fn after gate
print("\n=== MOVS #2 / MOVW #2 in START_NET fn 0x18e8028..0x18e8600 ===")
for j in range(0x18E8028, 0x18E8600, 2):
    hw = u16(j)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
        print(f"  {hex(j)} MOVS r{(hw>>8)&7},#2")
    mw = movw(j)
    if mw and mw[0] == 2:
        print(f"  {hex(j)} MOVW r{mw[1]},#2")

# --- SIT response complete helper 0x20e184e usage near NET with error 2 ---
# Known from dumps: BL 0x20e184e often with MOVS r0,#N as log level; may also be SIT complete
COMPLETE = 0x20E184E
print(f"\n=== BL complete-helper {hex(COMPLETE)} with preceding MOVS #2 in NET window ===")
for lo, hi in [(0x18E0000, 0x1920000), (0x1140000, 0x1160000), (0x1900000, 0x1A00000)]:
    p = lo
    while p + 4 < hi:
        if bl_target(p) == COMPLETE:
            # look back 0x20 for MOVS #2 or MOVW #2 or #0x704
            ctx = []
            for j in range(max(lo, p - 0x30), p, 2):
                hw = u16(j)
                if (hw & 0xFF00) == 0x2000:
                    ctx.append(f"MOVS#{hw&0xff}")
                mw = movw(j)
                if mw:
                    ctx.append(f"MOVW#{hex(mw[0])}")
            if any(x in ("MOVS#2", "MOVW#0x2", "MOVW#0x704") or x.startswith("MOVW#0x70") for x in ctx):
                print(f"  {hex(p)} ctx={ctx[-8:]}")
        p += 2

print("\nDONE")
