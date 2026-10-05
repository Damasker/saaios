#!/usr/bin/env python3
"""Full STATUS cluster 0x14fb3xx: CMP order Present/Pin1V/pin1/FCP."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000
SET_APP = 0x19916D2


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


def dump(start, end):
    o = start
    while o < end:
        hw = u16(o)
        extra = ""
        r, t, b = movw(o), movt(o), bl(o)
        if r:
            extra = f" MOVW r{r[1]},#{hex(r[0])}"
        if t:
            extra = f" MOVT r{t[1]},#{hex(t[0])}"
        if b is not None:
            extra = f" BL->{hex(b)}"
            if b == SET_APP:
                extra += " **SET_APP**"
        if (hw & 0xFF00) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            rt = (u16(o + 2) >> 12) & 0xF
            imm = u16(o + 2) & 0xFFF
            extra += f" LDRB.W r{rt},[r{hw&0xf},#{hex(imm)}]"
        if (hw & 0xFFF0) == 0xF880:
            extra += f" STRB.W [r{hw&0xf},#{hex(u16(o+2)&0xfff)}]"
        # LDRB r0,[rN,#imm] T1: 01111 imm5 rn rt
        if (hw & 0xF800) == 0x7800:
            extra += f" LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
        if hw in (0xBF08, 0xBF18, 0xBF28, 0xBF38):
            extra += f" IT"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# find PUSH.W before 0x14fb380
o = 0x14FB380
while o > 0x14FB000:
    if u16(o) == 0xE92D:
        print("prolog", hex(o))
        break
    o -= 2

print("\n=== FULL CLUSTER ===")
dump(0x14FB300, 0x14FB5E0)

print("\n=== nearby strings ===")
for n in [
    b"PIN_DISABLED",
    b"PIN DISABLED",
    b"pin disabled",
    b"PIN not required",
    b"PIN_NOT_REQUIRED",
    b"PIN required",
    b"CHV1 disabled",
    b"PS_DO",
    b"Pin1Verified",
    b"Present:",
    b"SIM STATUS update",
]:
    c = img.count(n)
    j = img.find(n)
    extra = ""
    if j >= 0:
        extra = img[j : img.find(b"\0", j)].decode("latin1", "replace")[:90]
    print(f"  {c} {n.decode()!r} {extra!r}")
