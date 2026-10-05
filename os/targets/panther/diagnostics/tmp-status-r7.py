#!/usr/bin/env python3
"""STATUS entry: prove r7 = PresentObj[0] before STRB +0xBF6."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()


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
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw & 0xf},#{hex(hw2 & 0xfff)}]"
            # MLA encoding FB0x xx00
            if (hw & 0xFFF0) == 0xFB00 and (hw2 & 0x00F0) == 0:
                rd = (hw2 >> 8) & 0xF
                rn = hw & 0xF
                rm = (hw2 >> 12) & 0xF
                ra = hw2 & 0xF
                extra = f" ;MLA r{rd},r{rn},r{rm},r{ra}"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw >> 8) & 7},#{hw & 0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw & 7},[r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw & 7},[r{(hw >> 3) & 7},#{(hw >> 6) & 0x1f}]"
            if (hw & 0xFFC0) == 0x4600:
                rd = (((hw >> 7) & 1) << 3) | (hw & 7)
                rm = (hw >> 3) & 0xF
                extra = f" ;MOV r{rd},r{rm}"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


dump(0x14FB322, 0x14FB390, "STATUS entry through +0xBF6 store")
# Decode STRB at 0x14fb380 precisely
hw, hw2 = u16(0x14FB380), u16(0x14FB382)
print(f"\nSTRB decode: hw={hw:04x} hw2={hw2:04x} Rn={hw&0xf} Rt={(hw2>>12)&0xf} imm={hex(hw2&0xfff)}")
