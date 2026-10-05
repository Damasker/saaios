#!/usr/bin/env python3
"""Follow-up: alternate STATUS entries + BF4 wide store + FN_B site."""
from __future__ import annotations
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/"
    "os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
mv = memoryview(IMG)
SET_APP = 0x19916D2


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(pc, h1, h2):
    if (h1 & 0xF800) != 0xF000 or (h2 & 0xD000) != 0xD000:
        return None
    s = (h1 >> 10) & 1
    imm10 = h1 & 0x3FF
    j1 = (h2 >> 13) & 1
    j2 = (h2 >> 11) & 1
    imm11 = h2 & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return (pc + 4 + imm32) & 0xFFFFFFFF


def decode(start, end):
    o = start
    while o < end:
        h = u16(o)
        if (h & 0xE000) == 0xE000 and (h & 0x1800) != 0:
            if o + 2 >= end:
                break
            h2 = u16(o + 2)
            tgt = bl_target(o, h, h2)
            if tgt is not None:
                tag = " SET_APP" if (tgt | 1) == (SET_APP | 1) else ""
                print(f"  0x{o:x}: BL 0x{tgt:x}{tag}")
                o += 4
                continue
            if (h & 0xFFF0) == 0xF890:
                print(f"  0x{o:x}: LDRB.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]")
                o += 4
                continue
            if (h & 0xFFF0) == 0xF880:
                print(f"  0x{o:x}: STRB.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]")
                o += 4
                continue
            if (h & 0xFFF0) == 0xF8C0:
                print(f"  0x{o:x}: STR.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]")
                o += 4
                continue
            if (h & 0xFFF0) == 0xF8A0:
                print(f"  0x{o:x}: STRH.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]")
                o += 4
                continue
            if (h & 0xF800) == 0xF000 and (h2 & 0xD000) == 0x9000:
                # B.W
                s = (h >> 10) & 1
                imm10 = h & 0x3FF
                j1 = (h2 >> 13) & 1
                j2 = (h2 >> 11) & 1
                imm11 = h2 & 0x7FF
                i1 = 1 - (j1 ^ s)
                i2 = 1 - (j2 ^ s)
                imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
                if s:
                    imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
                    if imm32 >= 0x80000000:
                        imm32 -= 0x100000000
                dest = (o + 4 + imm32) & 0xFFFFFFFF
                print(f"  0x{o:x}: B.W 0x{dest:x}")
                o += 4
                continue
            print(f"  0x{o:x}: t32 {h:04x}{h2:04x}")
            o += 4
            continue
        if (h & 0xF800) == 0x2000:
            print(f"  0x{o:x}: MOVS r{(h>>8)&7},#{h&0xFF}")
            o += 2
            continue
        if (h & 0xF800) == 0x2800:
            print(f"  0x{o:x}: CMP r{(h>>8)&7},#{h&0xFF}")
            o += 2
            continue
        if (h & 0xF000) == 0xD000 and (h & 0x0F00) not in (0x0E00, 0x0F00):
            conds = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
            imm8 = h & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            dest = o + 4 + (imm8 << 1)
            print(f"  0x{o:x}: B{conds[(h>>8)&0xF]} 0x{dest:x}")
            o += 2
            continue
        if (h & 0xF500) == 0xB100:
            print(f"  0x{o:x}: {'CBNZ' if h&0x800 else 'CBZ'} r{h&7}")
            o += 2
            continue
        if (h & 0xF800) == 0x7000:
            print(f"  0x{o:x}: STRB r{h&7},[r{(h>>3)&7},#{(h>>6)&0x1F}]")
            o += 2
            continue
        if (h & 0xF800) == 0x7800:
            print(f"  0x{o:x}: LDRB r{h&7},[r{(h>>3)&7},#{(h>>6)&0x1F}]")
            o += 2
            continue
        if h == 0x4770:
            print(f"  0x{o:x}: BX lr")
            o += 2
            continue
        print(f"  0x{o:x}: h16 {h:04x}")
        o += 2


def cstr_near(off, radius=0x100):
    """Find printable C strings whose pointer literals are nearby — crude: scan bytes."""
    found = []
    lo = max(0, off - radius)
    hi = min(len(IMG), off + radius)
    i = lo
    while i < hi:
        if 0x20 <= IMG[i] < 0x7F:
            j = i
            while j < hi and 0x20 <= IMG[j] < 0x7F:
                j += 1
            if j - i >= 6 and (j >= len(IMG) or IMG[j] == 0):
                s = IMG[i:j]
                if b"MMC" in s or b"SIM" in s or b"STATUS" in s or b"Present" in s or b"CDMA" in s or b"SADR" in s or b"LTE" in s:
                    found.append((i, s))
                i = j + 1
                continue
        i += 1
    return found


def movw_imm(o):
    h = u16(o)
    h2 = u16(o + 2)
    if (h & 0xFBF0) != 0xF240 or (h2 & 0x8000) != 0:
        return None
    i = (h >> 10) & 1
    return ((h & 0xF) << 12) | (i << 11) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)


def name_at_caller(bl_site, back=0x100):
    """Find MOVW/MOVT string pointers before BL."""
    names = []
    for b in range(max(0, bl_site - back), bl_site, 2):
        # MOVT then look back MOVW same rd
        h = u16(b)
        h2 = u16(b + 2) if b + 2 < len(IMG) else 0
        if (h & 0xFBF0) != 0xF2C0 or (h2 & 0x8000) != 0:
            continue
        i = (h >> 10) & 1
        imm_hi = ((h & 0xF) << 12) | (i << 11) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
        rd = (h2 >> 8) & 0xF
        for back2 in range(4, 80, 2):
            p = b - back2
            a0, a1 = u16(p), u16(p + 2)
            if (a0 & 0xFBF0) != 0xF240 or (a1 & 0x8000) != 0:
                continue
            if ((a1 >> 8) & 0xF) != rd:
                continue
            i2 = (a0 >> 10) & 1
            imm_lo = ((a0 & 0xF) << 12) | (i2 << 11) | (((a1 >> 12) & 7) << 8) | (a1 & 0xFF)
            imm = (imm_hi << 16) | imm_lo
            for cand in (imm, imm - 0x40000000 if imm >= 0x40000000 else None):
                if cand is None or cand < 0 or cand >= len(IMG):
                    continue
                if not (0x20 <= IMG[cand] < 0x7F):
                    continue
                e = IMG.find(b"\0", cand, cand + 120)
                s = IMG[cand : e if e >= 0 else cand + 80]
                if len(s) >= 4:
                    names.append(s)
            break
    return names


ENTRIES = [
    (0x14C5C78, 0x14FB6A0, "post-READY tail?"),
    (0x14C6670, 0x14FB322, "STATUS_WRAP→prolog"),
    (0x190A86C, 0x14FB4F0, "mid-STATUS A"),
    (0x1955E48, 0x14FB55C, "mid-STATUS B near READY"),
]

print("=== Alternate STATUS entry callers + names ===")
for bl, tgt, note in ENTRIES:
    names = name_at_caller(bl)
    print(f"\nBL@0x{bl:x} → 0x{tgt:x} ({note})")
    print(f"  names={names[:8]}")
    print(f"  --- decode target window ---")
    decode(tgt, tgt + 0x80)

print("\n=== STR.W +0xBF4 @0x19a539c context ===")
decode(0x19A5360, 0x19A53C0)
print("  caller names near site:")
# find BLs to nearby? just names in 0x100
print(" ", name_at_caller(0x19A539C, 0x120)[:6])

print("\n=== STRH +0xBF4 @0x1816bf2 / STRH +0xBF5 @0x18fe446 ===")
decode(0x1816BC0, 0x1816C20)
print("---")
decode(0x18FE420, 0x18FE470)

print("\n=== FN_A vs FN_B Present=2 candidates ===")
for label, o in [("FN_A", 0x14F6A16), ("cand_14f957c", 0x14F957C)]:
    print(f"\n-- {label} @0x{o:x} --")
    decode(o - 0x40, o + 0x20)
    # find BLs to function start — approximate function entry by scanning back for PUSH
    # Check MOVW #636c nearby
    for b in range(o - 0x80, o, 2):
        imm = movw_imm(b)
        if imm in (0x636C, 0x10):
            print(f"  MOVW #0x{imm:x} @0x{b:x}")

print("\n=== Path from mid-entries to SET#5: any path without LDRB bf6? ===")
# From 0x14fb4f0 and 0x14fb55c, walk linearly noting if we hit SET#5 before/after bf6 CMP
for start in (0x14FB4F0, 0x14FB55C, 0x14FB6A0):
    print(f"\nlinear from 0x{start:x} to 0x14fb640:")
    decode(start, 0x14FB640)

print("DONE")
