#!/usr/bin/env python3
"""Confirm: FCP DISABLED never STRB #1 to +20; find all STRB #1 [rN,#20] near verify; reset clears."""
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
    imm4 = hw & 0xF
    imm3 = (hw2 >> 12) & 7
    rd = (hw2 >> 8) & 0xF
    imm8 = hw2 & 0xFF
    return (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8, rd


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


# Pattern: MOVS Rd,#1 ; STRB Rd,[Rn,#20]  — Pin1Verified=1 candidate
# STRB T1 encoding: 0111 0 imm5 Rn Rt  => imm5=20=0x14 => (hw>>6)&0x1f == 0x14
print("=== MOVS#1 + STRB [rN,#20] across MAIN (Pin1Verified=1 candidates) ===")
setters = []
for o in range(MAIN_OFF, END - 6, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000 or (hw & 0xFF) != 1:
        continue
    rd = (hw >> 8) & 7
    # next insn STRB same rd, imm=20
    for gap in (2, 4):
        h2 = u16(o + gap)
        if (h2 & 0xF800) != 0x7000:
            continue
        rt = h2 & 7
        rn = (h2 >> 3) & 7
        imm5 = (h2 >> 6) & 0x1F
        if rt == rd and imm5 == 20:
            # nearby log id?
            logs = []
            for p in range(max(MAIN_OFF, o - 0x40), min(END, o + 0x20), 2):
                r = movw(p)
                if r and r[0] in (0x18E, 0x188, 0x20F, 0x17E, 0x167, 0x7AB, 0xBDA, 0x106A, 0x1CFD):
                    logs.append(f"{r[0]:#x}@{p:#x}")
            setters.append((o, rn, logs))
            print(f"SET+20 @0x{o:x}/va0x{va(o):x} STRB r{rd},[r{rn},#20] logs={logs}")

print(f"total_setters={len(setters)}")

# Pattern: MOVS#0 + STRB [rN,#20] — clear
print("\n=== MOVS#0 + STRB [rN,#20] (clear candidates) ===")
clears = []
for o in range(MAIN_OFF, END - 6, 2):
    hw = u16(o)
    if (hw & 0xFF00) != 0x2000 or (hw & 0xFF) != 0:
        continue
    rd = (hw >> 8) & 7
    for gap in (2, 4):
        h2 = u16(o + gap)
        if (h2 & 0xF800) != 0x7000:
            continue
        rt = h2 & 7
        rn = (h2 >> 3) & 7
        imm5 = (h2 >> 6) & 0x1F
        if rt == rd and imm5 == 20:
            logs = []
            for p in range(max(MAIN_OFF, o - 0x40), min(END, o + 0x20), 2):
                r = movw(p)
                if r and r[0] in (0x18E, 0x188, 0x20F, 0x17E, 0xBDA, 0x106A, 0x7AB):
                    logs.append(f"{r[0]:#x}")
            clears.append((o, rn, logs))
            if logs or len(clears) <= 20:
                print(f"CLR+20 @0x{o:x}/va0x{va(o):x} STRB [r{rn},#20] logs={logs}")

print(f"total_clears_shown_or_logged={len(clears)}")

# FCP USIM site 0x2b36700: any STRB #20?
print("\n=== STRB imm in FCP USIM window 0x2b366f0..0x2b36900 ===")
for o in range(0x2B366F0, 0x2B36900, 2):
    hw = u16(o)
    if (hw & 0xF800) == 0x7000:
        print(f"  STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}] @0x{o:x}")
    r = movw(o)
    if r and r[0] == 0x7AB:
        print(f"  LOG 0x7ab @0x{o:x}")

# first_pin function: who calls it / is it only from VerifyPin RSP?
# Find BL to roughly 0x1f044d6 (PUSH near first site)
print("\n=== BLs into first_pin cluster 0x1f044d0..0x1f04700 ===")
targets = set()
for o in range(MAIN_OFF, END - 4, 2):
    t = bl_target(o)
    if t and 0x1F044D0 <= t <= 0x1F04700:
        targets.add(t)
        # check if caller near vp_rsp log 0x17e
        near = []
        for p in range(max(MAIN_OFF, o - 0x80), o + 0x20, 2):
            r = movw(p)
            if r and r[0] in (0x17E, 0x167, 0x18E, 0x188, 0x20F, 0x404, 0x7AB):
                near.append(hex(r[0]))
        print(f"  BL@{o:#x}->0x{t:x} near_logs={near}")
print(f"unique_targets={sorted(hex(t) for t in targets)}")
