#!/usr/bin/env python3
"""Track base for +0xBF6 LDR/STR in STATUS; find MOVW#0xBF6; Pin1Verified link; FCP window stores."""
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


# 1) MOVW #0xBF6 anywhere
print("=== MOVW/MOVT involving 0xBF6 ===")
for o in range(MAIN_OFF, END - 4, 2):
    r = movw(o)
    if r and r[0] == 0xBF6:
        print(f"  MOVW r{r[1]},#0xBF6 @{o:#x}/va{va(o):#x}")
    # also 0x0BF6 as part of larger? skip

# 2) Full raw around STATUS STRB and each LDRB — decode fb1x 8000
print("\n=== RAW STATUS 0x14fb350..0x14fb3e0 ===")
for o in range(0x14FB350, 0x14FB3E0, 2):
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        tag = ""
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
            tag = f" STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#0xBF6]"
        elif (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF6:
            tag = f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#0xBF6]"
        elif (hw & 0xFBF0) == 0xF240:
            r = movw(o)
            tag = f" MOVW r{r[1]},#{r[0]:#x}" if r else ""
        elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            tag = f" BL->{bl_target(o):#x}"
        elif hw == 0xFB14 or hw == 0xFB15 or hw == 0xFB16 or hw == 0xFB04:
            tag = f" mulish {hw:04x} {hw2:04x}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{tag}")
        o  # noqa — loop still +2; fix below

# proper dump
print("\n=== Proper 2/4-byte dump 0x14fb350-0x14fb3e0 ===")
o = 0x14FB350
while o < 0x14FB3E0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        if (hw & 0xFFF0) == 0xF880:
            extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xFFF0) == 0xF890:
            extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            extra = f" ;BL->{bl_target(o):#x}"
        elif (hw & 0xFBF0) == 0xF240:
            r = movw(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}"
        elif (hw & 0xFBF0) == 0xF2C0:
            t = movt(o)
            if t:
                extra = f" ;MOVT r{t[1]},#{t[0]:#x}"
        elif (hw & 0xFFF0) == 0xF8C0:
            extra = f" ;STR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xFF70) == 0xF850:  # LDR.W imm
            extra = f" ;LDR.W?"
        # MLA/MUL family FB0x
        if (hw & 0xFF00) == 0xFB00:
            rn = hw & 0xF
            ra = (hw2 >> 12) & 0xF
            rd = (hw2 >> 8) & 0xF
            rm = hw2 & 0xF
            op = (hw >> 4) & 0xF
            # FB04 F00m = MUL; FB00 Ra Rd 0000 Rm = MLA when op nibble...
            if (hw2 & 0x00F0) == 0:
                if ra == 0xF:
                    extra = f" ;MUL r{rd},r{rn},r{rm}"
                else:
                    extra = f" ;MLA r{rd},r{rn},r{rm},r{ra}"
            else:
                extra = f" ;FB op rn={rn} ra={ra} rd={rd} rm={rm} hw2={hw2:#x}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF800) == 0x7800:
            extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        elif (hw & 0xFFC0) == 0x4600:
            extra = f" ;MOV"
        elif (hw & 0xFF00) == 0xBF00:
            extra = f" ;IT/NOP"
        print(f"  {o:#x}: {hw:04x}{extra}")
        o += 2

# Same for READY site
print("\n=== Proper dump READY 0x14fb5a8-0x14fb5d0 ===")
o = 0x14FB5A8
while o < 0x14FB5D0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        extra = ""
        if (hw & 0xFFF0) == 0xF890:
            extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
        elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
            extra = f" ;BL->{bl_target(o):#x}"
        elif (hw & 0xFBF0) == 0xF240:
            r = movw(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{r[0]:#x}"
        if (hw & 0xFF00) == 0xFB00:
            rn = hw & 0xF
            ra = (hw2 >> 12) & 0xF
            rd = (hw2 >> 8) & 0xF
            rm = hw2 & 0xF
            if (hw2 & 0x00F0) == 0:
                if ra == 0xF:
                    extra = f" ;MUL r{rd},r{rn},r{rm}"
                else:
                    extra = f" ;MLA r{rd},r{rn},r{rm},r{ra}"
        print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
        o += 4
    else:
        extra = ""
        if (hw & 0xFF00) == 0x2800:
            extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2000:
            extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xF000) == 0xD000 and (hw & 0x0F00) < 0x0E00:
            imm8 = hw & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            extra = f" ;Bcond->{o+4+(imm8<<1):#x}"
        print(f"  {o:#x}: {hw:04x}{extra}")
        o += 2

# 3) Pin1Verified STRB #20 sites — does same function also STRB #0xBF6 or MOVS #2 nearby?
print("\n=== Around Pin1Verified STRB sites 0x1f04576 / 0x1f046a4 ===")
for site in (0x1F04576, 0x1F046A4):
    print(f"\n-- site {site:#x} --")
    o = site - 0x40
    end = site + 0x80
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hw2&0xfff:#x}]"
            elif (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0xD000:
                extra = f" ;BL->{bl_target(o):#x}"
            elif (hw & 0xFBF0) == 0xF240:
                r = movw(o)
                if r:
                    note = f" ;LOG" if r[0] in (0x18E, 0x7AB, 0x106A) else ""
                    extra = f" ;MOVW r{r[1]},#{r[0]:#x}{note}"
            print(f"  {o:#x}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f"  {o:#x}: {hw:04x}{extra}")
            o += 2

# 4) FCP 0x7ab real sites — any STRB.W to large offsets or MOVS#2 STRB nearby
print("\n=== FCP 0x7ab real sites + nearby STRB.W/MOVS#2 ===")
for o in range(MAIN_OFF, END - 8, 2):
    r = movw(o)
    if not r or r[0] != 0x7AB:
        continue
    dense = False
    for p in range(o + 4, o + 32, 2):
        r2 = movw(p)
        if r2 and abs(r2[0] - 0x7AB) <= 1:
            dense = True
            break
    if dense:
        continue
    ctx = None
    for p in range(o - 24, o + 28, 2):
        t = movt(p)
        if t and 0x4000 <= t[0] <= 0x45FF:
            ctx = t[0]
            break
    if ctx is None:
        continue
    stores = []
    for p in range(o - 0x100, o + 0x200, 2):
        hw = u16(p)
        if (hw & 0xFFF0) == 0xF880:
            hw2 = u16(p + 2)
            stores.append(f"STRB.W [r{hw&0xf},#{hw2&0xfff:#x}] @{p:#x}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (0, 1, 2, 3, 5):
            # check if STRB T1 follows soon
            for q in range(p + 2, p + 12, 2):
                h = u16(q)
                if (h & 0xF800) == 0x7000:
                    stores.append(f"MOVS#{hw&0xff}+STRB[r{(h>>3)&7},#{(h>>6)&0x1f}] @{p:#x}")
                    break
                if (h & 0xFFF0) == 0xF880:
                    stores.append(f"MOVS#{hw&0xff}+STRB.W @{p:#x} off={u16(q+2)&0xfff:#x}")
                    break
    print(f"0x7ab @{o:#x} ctx={ctx:#x} stores_nearby={stores[:12]}")

# 5) Decode fb16 8000 before STRB Present — is it ADD that forms pointer?
print("\n=== Decode FB1x before STRB at 0x14fb37c ===")
hw, hw2 = u16(0x14FB37C), u16(0x14FB37E)
print(f"  {hw:04x} {hw2:04x}")
# Also 0x14fb3ce, 0x14fb518, 0x14fb5b8
for addr in (0x14FB37C, 0x14FB3CE, 0x14FB518, 0x14FB5B8, 0x14FB42E, 0x14FB482):
    hw, hw2 = u16(addr), u16(addr + 2)
    rn = hw & 0xF
    ra = (hw2 >> 12) & 0xF
    rd = (hw2 >> 8) & 0xF
    rm = hw2 & 0xF
    print(f"  @{addr:#x}: {hw:04x} {hw2:04x}  interpret MLA/MUL rn={rn} ra={ra} rd={rd} rm={rm}")
