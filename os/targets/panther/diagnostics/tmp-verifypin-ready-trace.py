#!/usr/bin/env python3
"""Trace VerifyPin -> Present=2 -> SET_APP#5 without CDMA; factory cold-boot SIT order."""
import struct
import re
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def bl(o):
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


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8C0:
                extra = f" ;STR.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB [r{hw & 7},r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xFFC0) == 0x4600:
                rd = (((hw >> 7) & 1) << 3) | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80:
                    imm -= 0x100
                tgt = o + 4 + imm * 2
                names = {
                    0: "EQ",
                    1: "NE",
                    2: "CS",
                    3: "CC",
                    4: "MI",
                    5: "PL",
                    6: "VS",
                    7: "VC",
                    8: "HI",
                    9: "LS",
                    10: "GE",
                    11: "LT",
                    12: "GT",
                    13: "LE",
                }
                if cond < 14:
                    extra = f" ;B{names[cond]}->{hex(tgt)}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400:
                    imm -= 0x800
                extra = f" ;B->{hex(o + 4 + imm * 2)}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


def find_movw_movt(sva, limit=None):
    lo = sva & 0xFFFF
    hi = (sva >> 16) & 0xFFFF
    hits = []
    end = len(img) - 8 if limit is None else min(len(img) - 8, limit)
    o = MAIN
    while o < end:
        r = movw(o)
        if r and r[0] == lo:
            for d in range(0, 24, 2):
                t = movt(o + d)
                if t and t[1] == r[1] and t[0] == hi:
                    hits.append(o)
                    break
        o += 2
    return hits


def str_va(needle):
    j = img.find(needle)
    if j < 0:
        return None, None
    return j, VA + (j - MAIN)


# --- A-ish: strings for cold-boot / SIM_INFO ---
print("=== strings ===")
for name in [
    b"First PIN1 Verification",
    b"NS_USIM_VERIFYPIN_RSP",
    b"A[USIM_%d] >> DetermineSimStatus",
    b"USIM <== SIM_INFO_REQ",
    b"sitSendNsSimInfoReq",
    b"SET_APP",
    b"Pin1Verified",
    b"SIM_PRESENT_IND",
    b"SIM_START_IND",
    b"SIM_PIN_STATUS_IND",
]:
    j, sva = str_va(name)
    print(f"{name!r}: off={hex(j) if j is not None else None} VA={hex(sva) if sva else None}")

# First PIN1 Verification xrefs
_, fp_va = str_va(b"First PIN1 Verification")
if fp_va:
    hits = find_movw_movt(fp_va)
    print(f"\nFirst PIN1 MOVW+MOVT hits: {len(hits)} {[hex(h) for h in hits]}")
    for h in hits[:4]:
        dump(h - 0x80, h + 0xA0, f"FirstPIN1@{hex(h)}")

# DetermineSimStatus
_, ds_va = str_va(b"A[USIM_%d] >> DetermineSimStatus")
if ds_va:
    hits = find_movw_movt(ds_va)
    print(f"\nDetermineSimStatus hits: {len(hits)} {[hex(h) for h in hits[:20]]}")
    for h in hits[:4]:
        dump(h - 0xA0, h + 0x80, f"DetSim@{hex(h)}")

# SIM_INFO_REQ
_, si_va = str_va(b"USIM <== SIM_INFO_REQ")
if si_va:
    hits = find_movw_movt(si_va)
    print(f"\nSIM_INFO_REQ hits: {len(hits)} {[hex(h) for h in hits[:20]]}")
    for h in hits[:3]:
        dump(h - 0x40, h + 0xC0, f"SimInfo@{hex(h)}")

# Known FirstPIN sites from prior work
for addr in (0x1F0456C, 0x1F0469A):
    dump(addr - 0x40, addr + 0x80, f"known FirstPIN@{hex(addr)}")

# SET_APP READY gate region
dump(0x14FB580, 0x14FB620, "SET_APP READY region")

# FN_A Present=2
dump(0x14F6A00, 0x14F6A40, "FN_A Present=2")

# STATUS Present copy
dump(0x14FB360, 0x14FB400, "STATUS +0xBF6 copy")
