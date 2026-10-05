#!/usr/bin/env python3
from pathlib import Path
import struct

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000


def fo(va):
    return va - VA + MAIN


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def decode_bw(o):
    hw, hw2 = u16(o), u16(o + 2)
    print(f" {hex(o)} {hw:04x} {hw2:04x}")
    # B<cond>.W
    if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x8000:
        s = (hw >> 10) & 1
        cond = (hw >> 6) & 0xF
        j1 = (hw2 >> 13) & 1
        j2 = (hw2 >> 11) & 1
        imm6 = hw & 0x3F
        imm11 = hw2 & 0x7FF
        i1 = ~(j1 ^ s) & 1
        i2 = ~(j2 ^ s) & 1
        imm32 = (s << 20) | (i1 << 19) | (i2 << 18) | (imm6 << 12) | (imm11 << 1)
        if s:
            imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
            if imm32 >= 0x80000000:
                imm32 -= 0x100000000
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
        print(f"  B{names.get(cond,'?')}.W -> {hex(o+4+imm32)} cond={cond}")


decode_bw(0x18E8330)

for va in (0x44F39DDC, 0x44F3A138, 0x44F3F0EC, 0x44F3F154, 0x44F5B2CC):
    o = fo(va)
    if 0 <= o < len(img):
        s = img[o : o + 80]
        end = 0
        while end < len(s) and 32 <= s[end] < 127:
            end += 1
        print(hex(va), s[:end].decode("ascii", "replace"))
    else:
        print(hex(va), "OOB", hex(o))

# SET_APP#1: more context — CMP Present?
print("\n#1 more before")
o = 0x146A9E0
while o < 0x146AAB8:
    hw = u16(o)
    if (hw & 0xFFF0) == 0xF890:
        print(hex(o), "LDRB", hex(u16(o + 2) & 0xFFF))
    if (hw & 0xFF00) == 0x2800:
        print(hex(o), "CMP", hw & 0xFF)
    o += 2
