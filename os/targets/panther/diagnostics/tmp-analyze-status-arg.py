#!/usr/bin/env python3
import struct
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


print("=== around STATUS call 0x14c6670 ===")
o = 0x14C6620
while o < 0x14C66A0:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        parts = []
        r, t, b = movw(o), movt(o), bl(o)
        if r:
            parts.append(f"MOVW r{r[1]},#{hex(r[0])}")
        if t:
            parts.append(f"MOVT r{t[1]},#{hex(t[0])}")
        if b:
            parts.append(f"BL->{hex(b)}")
        if (hw & 0xFFF0) == 0xF890:
            parts.append(f"LDRB.W [r{hw & 0xf},#{hex(hw2 & 0xfff)}]")
        print(f"  {hex(o)}: {' | '.join(parts) if parts else f'{hw:04x} {hw2:04x}'}")
        o += 4
    else:
        parts = []
        if (hw & 0xFF00) == 0x2000:
            parts.append(f"MOVS r{(hw >> 8) & 7},#{hw & 0xff}")
        if (hw & 0xFFC0) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            parts.append(f"MOV r{rd},r{rm}")
        if (hw & 0xF800) == 0x6800:
            parts.append(f"LDR r{hw & 7},[r{(hw >> 3) & 7},#{((hw >> 6) & 0x1f) * 4}]")
        if parts:
            print(f"  {hex(o)}: {' | '.join(parts)}")
        o += 2

fn = None
for p in range(0x14C6670, 0x14C6670 - 0x200, -2):
    if u16(p) in (0xB570, 0xB5F0, 0xB5B0) or (u16(p) & 0xFFF0) == 0xE92D:
        fn = p
        break
print("fn", hex(fn) if fn else None)
cs = [o for o in range(0x14B0000, 0x14D0000, 2) if bl(o) == fn] if fn else []
print("callers", list(map(hex, cs)))
for c in cs[:8]:
    names = []
    for p in range(c - 0x50, c + 8, 2):
        r = movw(p)
        if not r:
            continue
        for q in range(p - 16, p + 20, 2):
            t = movt(q)
            if t and t[1] == r[1] and 0x4100 <= t[0] <= 0x4110:
                v = (t[0] << 16) | r[0]
                oo = MAIN + (v - VA)
                s = b""
                while oo < len(img) and 32 <= img[oo] < 127:
                    s += bytes([img[oo]])
                    oo += 1
                if s:
                    names.append(s.decode())
    print(f"  {hex(c)} {names}")

print("\n=== 0x14fb7f0..0x14fb860 getobj#4 ===")
o = 0x14FB7F0
while o < 0x14FB860:
    hw = u16(o)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        hw2 = u16(o + 2)
        parts = []
        r, b = movw(o), bl(o)
        if r:
            parts.append(f"MOVW #{hex(r[0])}")
        if b:
            parts.append(f"BL->{hex(b)}")
        if (hw & 0xFFF0) == 0xF880:
            parts.append(f"STRB.W #{hex(hw2 & 0xfff)}")
        if (hw & 0xFFF0) == 0xF890:
            parts.append(f"LDRB.W #{hex(hw2 & 0xfff)}")
        print(f"  {hex(o)}: {' | '.join(parts) if parts else f'{hw:04x}'}")
        o += 4
    else:
        if (hw & 0xFF00) == 0x2000:
            print(f"  {hex(o)}: MOVS #{hw & 0xff}")
        if (hw & 0xFFC0) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            print(f"  {hex(o)}: MOV r{rd},r{rm}")
        o += 2

# Does STATUS use getobj#4? Scan STATUS fn for BL getobj / MOV r5 from getobj
print("\n=== STATUS fn refs to getobj 0x20ea040 / LDRB [r5,#0] ===")
for o in range(0x14FB322, 0x14FB600, 2):
    b = bl(o)
    if b in (0x20EA040, 0x14F692C, 0x14F6D02):
        print(f"  {hex(o)} BL->{hex(b)}")
    if u16(o) == 0x782F:  # LDRB r7,[r5,#0]
        print(f"  {hex(o)} LDRB r7,[r5,#0]")
    if u16(o) == 0x2004 and bl(o + 2) == 0x20EA040:
        print(f"  {hex(o)} MOVS#4 + getobj")
