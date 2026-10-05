#!/usr/bin/env python3
import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


print("=== LDR/MOV into r9 before Present=2 ===")
for o in range(0x14F692C, 0x14F6A14, 2):
    hw = u16(o)
    if (hw & 0xFFF0) == 0xF890:
        hw2 = u16(o + 2)
        if ((hw2 >> 12) & 0xF) == 9:
            print(f"  LDRB.W r9,[r{hw&0xf},#{hw2&0xfff:#x}] @{o:#x}")
    if (hw & 0xFFF0) == 0xF8D0:
        hw2 = u16(o + 2)
        if ((hw2 >> 12) & 0xF) == 9:
            print(f"  LDR.W r9,[r{hw&0xf},#{hw2&0xfff:#x}] @{o:#x}")
    if (hw & 0xFF00) == 0x4600:
        rd = ((hw >> 7) & 1) << 3 | (hw & 7)
        rm = (hw >> 3) & 0xF
        if rd == 9:
            print(f"  MOV r9,r{rm} @{o:#x}")
    # LDRB T1 into r1.. high regs need LDRB.W
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        pass

print("\n=== fn prologue 0x14f692c (r9 source likely arg) ===")
for o in range(0x14F692C, 0x14F6980, 2):
    hw = u16(o)
    extra = ""
    if (hw & 0xFF00) == 0x4600:
        rd = ((hw >> 7) & 1) << 3 | (hw & 7)
        rm = (hw >> 3) & 0xF
        extra = f" ;MOV r{rd},r{rm}"
    if (hw & 0xFFF0) == 0xE92D:
        extra = f" ;PUSH.W {u16(o+2):04x}"
    print(f"  {o:#x}: {hw:04x}{extra}")

# 468b = MOV r11, r1; 4681 = MOV r9, r0?
# 0100 0110 10 001 001 = 4689 MOV r9,r1?
print("decode 468b:", end=" ")
hw = 0x468B
rd = ((hw >> 7) & 1) << 3 | (hw & 7)
rm = (hw >> 3) & 0xF
print(f"MOV r{rd},r{rm}")
hw = 0x4681
rd = ((hw >> 7) & 1) << 3 | (hw & 7)
rm = (hw >> 3) & 0xF
print(f"4681 MOV r{rd},r{rm}")

# log ids near 0x106a string
print("\n=== nearby log hdrs ===")
base = 0x4D4F640
for off in range(base - 0x400, base + 0x200):
    w = struct.unpack_from("<I", img, off)[0]
    if (w & 0xFF) == 0x44:
        lid = (w >> 8) & 0xFFFF
        if 0x1060 <= lid <= 0x1070:
            s = off + 8
            if 32 <= img[s] < 127:
                end = img.find(b"\0", s)
                print(f"  lid={lid:#x}: {img[s:end].decode()[:120]}")

# Caller 0x14c37f2 context
print("\n=== caller of Present=2 fn @0x14c37f2 ===")
for o in range(0x14C37C0, 0x14C3820, 2):
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        print(f"  {o:#x}: {hw:04x} {hw2:04x}")
    else:
        print(f"  {o:#x}: {hw:04x}")
