#!/usr/bin/env python3
"""Analyze all MOVW #0xBF6 sites: stores of what values; link to VerifyPin/FCP."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()

BF6_SITES = [
    0x1157DA2,
    0x1201FFE,
    0x1BDA9E6,
    0x1C2F8AE,
    0x1D5E846,
    0x220753C,
    0x2207F7E,
    0x2299436,
    0x22BD47C,
    0x230B862,
    0x2F3806E,
    0x3222160,
]


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


def dump(start, length):
    o = start
    end = start + length
    out = []
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r = movw(o)
            if r:
                note = ""
                if r[0] in (0x7AB, 0x18E, 0x106A, 0xC6B, 0xC5B, 0xBDA, 0x2C5E, 0xBF6):
                    note = " <<LOG/OFF"
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFFF0) == 0xF800:  # STRB register
                # F80n | Rt 0000 00=imm? actually STRB Rt,[Rn,Rm]
                extra = f" ;STRB.W reg? {hw:04x} {hw2:04x}"
            elif (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            elif (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            elif (hw & 0xFFE0) == 0xEB00 or (hw & 0xFFE0) == 0xEB10:
                extra = f" ;ADD.W"
            elif (hw & 0xF800) == 0xF100 or (hw & 0xF800) == 0xF200:
                extra = f" ;ADD/SUB.W imm"
            out.append(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7000:
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xF800) == 0x5400:
            # STRB Rt,[Rn,Rm] T1: 0101 010 Rm Rn Rt
            extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},r{(hw>>6)&7}]"
        elif (hw & 0xF800) == 0x5C00:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},r{(hw>>6)&7}]"
        elif (hw & 0xFFC0) == 0x4400:
            extra = f" ;ADD"
        elif (hw & 0xF800) == 0x1800:
            extra = f" ;ADDS"
        elif (hw & 0xFF00) == 0x4600:
            extra = f" ;MOV"
        out.append(f"  {o:#x}: {hw:04x}{extra}")
        o += 2
    return "\n".join(out)


for site in BF6_SITES:
    print(f"\n======== MOVW #0xBF6 @{site:#x}/va{va(site):#x} ========")
    print(dump(site - 0x30, 0x90))

# Also: after Pin1Verified STRB, is there a call that might set +0xBF6?
# Search for STRB of value 2 with ADD involving 0xBF6 pattern within ±0x200 of 0x18e sites
print("\n\n=== 0x18e sites: MOVS #2 within ±0x100 + any BF6 ===")
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0x18E:
        continue
    dense = False
    for p in range(o + 4, o + 24, 2):
        r2 = movw(p)
        if r2 and abs(r2[0] - 0x18E) <= 2:
            dense = True
            break
    if dense:
        continue
    # check ctx
    ctx = None
    for p in range(o - 20, o + 20, 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            if 0x4000 <= imm <= 0x45FF:
                ctx = imm
                break
    if ctx is None:
        continue
    has_bf6 = False
    movs2 = []
    for p in range(o - 0x80, o + 0x120, 2):
        if movw(p) and movw(p)[0] == 0xBF6:
            has_bf6 = True
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            movs2.append(p)
    print(f"0x18e @{o:#x} ctx={ctx:#x} has_bf6={has_bf6} MOVS#2={list(map(hex,movs2))}")

# Search string near table 0x48c649b8
print("\n=== ptr 0x48c649b8 xrefs (MOVW/MOVT) used with BF6 ===")
# Find MOVW #0x49b8 + MOVT #0x48c6
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0x49B8:
        continue
    # look for MOVT #0x48c6 same reg nearby
    for p in range(o - 8, o + 12, 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFBF0) == 0xF2C0 and (hw2 & 0x8000) == 0:
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            rd = (hw2 >> 8) & 0xF
            if imm == 0x48C6 and rd == r[1]:
                print(f"  table load r{r[1]} @{o:#x}/va{va(o):#x}")
                break
