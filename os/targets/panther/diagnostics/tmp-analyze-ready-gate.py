#!/usr/bin/env python3
"""Decode STATUS_UPD gate: raw halfwords + what r0 is at READY/PIN/PUK/#4."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = PATH.read_bytes()
VA_BASE = 0x40010000
MAIN_OFF = 0x16C10


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def dump_raw(start, end):
    """Print every halfword with Thumb decode hints."""
    o = start
    while o < end:
        hw = u16(o)
        note = ""
        # IT
        if (hw & 0xFF00) == 0xBF00 and (hw & 0x000F) != 0:
            note = f"  ; IT mask={hw&0xff:02x}"
        elif (hw & 0xFF00) == 0x2000:
            note = f"  ; MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            note = f"  ; CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7800:
            note = f"  ; LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x7000:
            note = f"  ; STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x6800:
            note = f"  ; LDR r{hw&7},[r{(hw>>3)&7},#{((hw>>6)&0x1f)*4}]"
        elif (hw & 0xFFE0) == 0x2000:  # already covered
            pass
        elif (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            note = f"  ; wide {hw:04x} {hw2:04x}"
            # MOVW/MOVT
            if (hw & 0xFBF0) == 0xF240 and (hw2 & 0x8000) == 0:
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                note = f"  ; MOVW r{(hw2>>8)&0xf},#{imm:#x}"
            elif (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                note = f"  ; MOVT r{(hw2>>8)&0xf},#{imm:#x}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                note = "  ; BL"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x8000:
                note = "  ; B.W / BLX?"
            # LDR.W / etc
            print(f"{o:#010x}: {hw:04x} {hw2:04x}{note}")
            o += 4
            continue
        elif (hw & 0xF000) == 0xD000 and (hw & 0x0F00) < 0x0E00:
            cond = (hw >> 8) & 0xF
            imm8 = hw & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            tgt = o + 4 + (imm8 << 1)
            note = f"  ; Bcond->{tgt:#x}"
        elif (hw & 0xF800) == 0xE000:
            imm11 = hw & 0x7FF
            if imm11 >= 0x400:
                imm11 -= 0x800
            tgt = o + 4 + (imm11 << 1)
            note = f"  ; B->{tgt:#x}"
        elif (hw & 0xFFC0) == 0x4240:
            note = f"  ; RSBS/NEGS"
        elif hw == 0xBF00:
            note = "  ; NOP"
        elif (hw & 0xFF00) == 0x4600:
            note = f"  ; MOV/ADD low"
        print(f"{o:#010x}: {hw:04x}{note}")
        o += 2


# Critical windows
print("=== RAW PIN/PUK/READY decision 0x14fb340..0x14fb5e0 ===")
dump_raw(0x14FB340, 0x14FB5E0)

# What is 0x106b string?
s = img.find(b"SIM STATUS update")
# find nearby strings with 0x106b
print("\n=== strings near STATUS update ===")
# scan for log id 0x106b via shannon pack
for off in range(0x4D4F000, 0x4D50000):
    pass
# search strings containing Verified or STATUS update
import re
chunk = img[0x4D4F000:0x4D51000]
# find printable runs
i = 0
while i < len(chunk):
    if 32 <= chunk[i] < 127:
        j = i
        while j < len(chunk) and 32 <= chunk[j] < 127:
            j += 1
        if j - i >= 20:
            t = chunk[i:j].decode()
            if "STATUS" in t or "Verified" in t or "Present" in t or "Pin1" in t or "MePer" in t:
                abs_off = 0x4D4F000 + i
                hdr = struct.unpack_from("<I", img, abs_off - 8)[0]
                print(f"  {abs_off:#x} hdr={hdr:#x}: {t[:120]}")
        i = j
    else:
        i += 1

# Decode: does CMP #3 before SET_APP #4 compare Pin1Verified or pin1 status?
# Look at LDRB sources just before 0x14fb520
print("\n=== RAW around CMP#3->SET#4 0x14fb4f0..0x14fb530 ===")
dump_raw(0x14FB4F0, 0x14FB530)

# Search whole MAIN for SET_APP with MOVS #5 again confirming uniqueness
# Also check if sitSetPin1Status DISABLED (#3) ever BLs to SET_APP READY
print("\n=== sitSetPin1 / pin status #3 near SET_APP? (search MOVW 0xc6b then SET_APP in same fn) ===")
# For each 0xc6b real site, scan forward 0x400 for SET_APP


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


SET_APP = 0x19916D2
# In STATUS cluster, list every SET_APP with imm and preceding CMP
print("\n=== SET_APP sites in 0x14fb200..0x14fb700 with gate CMP ===")
for o in range(0x14FB200, 0x14FB700, 2):
    if bl_target(o) != SET_APP:
        continue
    imm = None
    for b in range(2, 16, 2):
        hw = u16(o - b)
        if (hw & 0xFF00) == 0x2000:
            imm = hw & 0xFF
            break
    # look back for CMP
    cmps = []
    for b in range(2, 0x40, 2):
        hw = u16(o - b)
        if (hw & 0xFF00) == 0x2800:
            cmps.append(f"CMP r{(hw>>8)&7},#{hw&0xff} @{o-b:#x}")
        if (hw & 0xF000) == 0xD000 and (hw & 0x0F00) < 0x0E00:
            cmps.append(f"Bcond @{o-b:#x}")
            break
    print(f"SET_APP@{o:#x} imm={imm} gates={cmps[:6]}")
