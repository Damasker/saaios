#!/usr/bin/env python3
"""Trace First PIN1 Verification → Present/READY; falsify CDMA-only model."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, END, VA = 0x16C10, 0x16C10 + 0x05917ACC, 0x40010000
SET_APP, FN_A, WRAP_SIM = 0x19916D2, 0x14F692C, 0x14F6D02


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


def dump(start, end, label=""):
    if label:
        print(f"\n=== {label} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b = movw(o), bl(o)
            if r:
                note = ""
                if r[0] in (0x18E, 0x106A, 0x1068, 0x7AB, 0x17E, 0x20F, 0xBDA):
                    note = " LOG"
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}{note}"
            if b:
                tag = ""
                if b == SET_APP:
                    tag = " <<SET_APP"
                elif b == FN_A:
                    tag = " <<FN_A"
                elif b == WRAP_SIM:
                    tag = " <<WRAP_SIM"
                elif b == 0x14FB322:
                    tag = " <<STATUS"
                elif b == 0x14C6626:
                    tag = " <<STATUS_WRAP"
                extra = f" ;BL->{hex(b)}{tag}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                rd = ((hw >> 7) & 1) << 3 | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2


# Doc sites for First PIN1 Verification
for lab, start, end in (
    ("FirstPIN1 SIT0 0x1f0456c", 0x1F04540, 0x1F04620),
    ("FirstPIN1 SIT0 b 0x1f0469a", 0x1F04670, 0x1F04730),
):
    dump(start, end, lab)

# Search MOVW #0x18e near BL SET_APP / FN_A / Present STRB
print("\n=== sites with log 0x18e and nearby interesting BL/STRB ===")
for o in range(MAIN, END - 4, 2):
    r = movw(o)
    if not r or r[0] != 0x18E:
        continue
    # window
    interesting = []
    for p in range(o, min(o + 0x80, END - 4), 2):
        b = bl(p)
        if b in (SET_APP, FN_A, WRAP_SIM, 0x14FB322, 0x14C6626, 0x14F9108, 0x14F9416):
            interesting.append(f"{hex(p)}->{hex(b)}")
        hw = u16(p)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(p + 2)
            if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) in (0, 0xBF6):
                interesting.append(f"{hex(p)} STRB.W #{hex(hw2&0xfff)}")
            continue
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) in (0, 20):
            interesting.append(f"{hex(p)} STRB #{(hw>>6)&0x1f}")
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) in (2, 5):
            interesting.append(f"{hex(p)} MOVS #{hw&0xff}")
    if interesting:
        print(f"  0x18e@{hex(o)}: {interesting[:12]}")

# Re-dump READY gate: full STATUS from Present copy through SET_APP#5
# Check if Pin1Verified (+0xBF5 or [r5,#1]) is also required
dump(0x14FB3D0, 0x14FB5E0, "STATUS Present→READY full")

# Who calls sitSendNsSimInfoReq equivalent - search string via packed id won't work
# Search BL from FirstPIN cluster
print("\n=== callees from FirstPIN cluster 0x1f04540..0x1f04780 ===")
cals = set()
for o in range(0x1F04540, 0x1F04780, 2):
    b = bl(o)
    if b:
        cals.add(b)
for b in sorted(cals):
    tag = ""
    if b == SET_APP:
        tag = " SET_APP"
    if b == FN_A:
        tag = " FN_A"
    if b == 0x14FB322:
        tag = " STATUS"
    print(f"  {hex(b)}{tag}")

# Does FirstPIN or SimInfo set Present via STRB #0 = 2?
print("\n=== STRB Present=2 in 0x1f04000..0x1f05000 and 0x1d20000..0x1d30000 ===")
for lo, hi in [(0x1F04000, 0x1F05000), (0x1D26000, 0x1D28000), (0x1D4E000, 0x1D50000)]:
    for o in range(lo, hi, 2):
        if u16(o) != 0x2002:
            continue
        for p in range(o + 2, min(o + 12, hi), 2):
            hw = u16(p)
            if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0 and (hw & 7) == 0:
                print(f"  {hex(o)} MOVS#2 STRB [r{(hw>>3)&7},#0] @{hex(p)}")
            if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
                hw2 = u16(p + 2)
                if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0 and ((hw2 >> 12) & 0xF) == 0:
                    print(f"  {hex(o)} MOVS#2 STRB.W [r{hw&0xf},#0] @{hex(p)}")
                break

print("\nDONE")
