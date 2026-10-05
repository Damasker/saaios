#!/usr/bin/env python3
"""Disasm PresentObj getobj(0x636c) sites + filter Present=2 to same obj."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, END, VA = 0x16C10, 0x16C10 + 0x05917ACC, 0x40010000
GETOBJ = 0x20EA040


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


def dump(start, end, label=""):
    if label:
        print(f"\n=== {label} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, b = movw(o), movt(o), bl(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b:
                extra = f" ;BL->{hex(b)}"
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
            if extra or True:
                print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2


sites = [
    (0x14F6940, 0x14F6956, "FN_A"),
    (0x1551CF4, 0x1551D0A, "0x1551"),
    (0x1551F8A, 0x1551FA0, "0x1551b"),
    (0x1553416, 0x155342C, "0x1553"),
    (0x1A175EA, 0x1A17600, "0x1a17"),
    (0x1A347E4, 0x1A347F0, "0x1a34"),
    (0x1A55204, 0x1A5521A, "0x1a55"),
]

for imm, go, lab in sites:
    # dump from 0x30 before getobj to 0x80 after
    dump(go - 0x28, go + 0x60, f"{lab} getobj@{hex(go)}")

# Focus: after getobj, which register holds obj, any STRB #0 with imm 0/1/2/3?
print("\n=== Post-getobj Present field stores (STRB #0 with MOVS 0-3) ===")
for imm, go, lab in sites:
    # find MOV rd,r0 after getobj (4680 = mov r8,r0 etc)
    obj_reg = None
    for p in range(go + 4, go + 12, 2):
        hw = u16(p)
        if (hw & 0xFFC0) == 0x4600:
            rd = ((hw >> 7) & 1) << 3 | (hw & 7)
            rm = (hw >> 3) & 0xF
            if rm == 0:
                obj_reg = rd
                break
        if (hw & 0xFF00) == 0x0000:  # MOV low
            pass
    # also 4680 encoding: MOV r8,r0 is 0x4680
    if u16(go + 4) == 0x4680:
        obj_reg = 8
    print(f"{lab}: obj_reg={obj_reg}")
    # scan forward 0x100 for STRB to obj_reg #0
    for p in range(go + 4, go + 0x120, 2):
        hw = u16(p)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(p + 2)
            if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
                rn = hw & 0xF
                rt = (hw2 >> 12) & 0xF
                if obj_reg is None or rn == obj_reg:
                    # look back for MOVS rt,#imm
                    val = None
                    for q in range(max(go, p - 12), p, 2):
                        h = u16(q)
                        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == (rt if rt < 8 else -1):
                            val = h & 0xFF
                        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == 0 and rt == 0:
                            val = h & 0xFF
                    print(f"  {hex(p)} STRB.W r{rt},[r{rn},#0] val~{val}")
            continue
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) == 0:
            rn = (hw >> 3) & 7
            rt = hw & 7
            if obj_reg is None or rn == obj_reg or (obj_reg and obj_reg < 8 and rn == obj_reg):
                val = None
                for q in range(max(go, p - 12), p, 2):
                    h = u16(q)
                    if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == rt:
                        val = h & 0xFF
                if val is not None and val <= 3:
                    print(f"  {hex(p)} STRB r{rt},[r{rn},#0] val={val}")

# 0x14f900a candidate near FN_B
dump(0x14F8FE0, 0x14F9040, "near 0x14f900a")

# STATUS uses PresentObj from caller — who builds default?
# Search memset/zero of getobj result in FN_A path and 0x14fb82e
dump(0x14FB82A, 0x14FB8A0, "STATUS-side getobj#4 @0x14fb83a")

# INSERT / SIM_PRESENT_IND handlers that might set Present
print("\n=== SIM_PRESENT_IND / INSERT strings VA ===")
for n in (
    b"SIM_PRESENT_IND",
    b"/SIM_PRESENT_IND",
    b"MM_SIM_PRESENT_IND_Handler",
    b"SimPresent : %d -> %d",
    b"USIM <== SIM_INIT_REQ",
    b"SIM_INIT_COMPLETED",
    b"PIN_DISABLED",
):
    j = img.find(n)
    if j < 0:
        print(n, None)
        continue
    v = VA + (j - MAIN) if j >= MAIN else None
    print(f"  @{hex(j)} VA={hex(v) if v else '?'} {n.decode()}")

print("\nDONE")
