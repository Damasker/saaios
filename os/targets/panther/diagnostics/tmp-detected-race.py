#!/usr/bin/env python3
"""Decode STATUS PIN-gate path and SET#1 function; Present inference."""
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, VA = 0x16C10, 0x40010000

def half(va):
    o = MAIN + (va - VA)
    return img[o] | (img[o + 1] << 8)

def word(va):
    return half(va) | (half(va + 2) << 16)

def bl_target(va):
    hw = half(va)
    hw2 = half(va + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    S = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    J1 = (hw2 >> 13) & 1
    J2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    I1 = 1 - (J1 ^ S)
    I2 = 1 - (J2 ^ S)
    imm = (S << 24) | (I1 << 23) | (I2 << 22) | (imm10 << 12) | (imm11 << 1)
    if S:
        imm -= 1 << 25
    return (va + 4 + imm) & 0xFFFFFFFF

def dump(lo, hi, label):
    print(f"\n=== {label} {hex(lo)}..{hex(hi)} ===")
    va = lo
    while va < hi:
        hw = half(va)
        # 32-bit
        if va + 3 < hi + 4:
            hw2 = half(va + 2)
            tgt = bl_target(va)
            if tgt is not None:
                mark = " **SET_APP**" if tgt == 0x19916D2 else ""
                mark += " **GET_APP**" if tgt == 0x18EC8C0 else ""
                print(f"  {hex(va)}: BL -> {hex(tgt)}{mark}")
                va += 4
                continue
            if (hw & 0xFFF0) == 0xF890:
                rn = hw & 0xF
                rt = (hw2 >> 12) & 0xF
                imm = hw2 & 0xFFF
                print(f"  {hex(va)}: LDRB.W r{rt},[r{rn},#{hex(imm)}]")
                va += 4
                continue
            if (hw & 0xFFF0) == 0xF880:
                rn = hw & 0xF
                rt = (hw2 >> 12) & 0xF
                imm = hw2 & 0xFFF
                print(f"  {hex(va)}: STRB.W r{rt},[r{rn},#{hex(imm)}]")
                va += 4
                continue
            # B.W
            if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x9000:
                S = (hw >> 10) & 1
                imm10 = hw & 0x3FF
                J1 = (hw2 >> 13) & 1
                J2 = (hw2 >> 11) & 1
                imm11 = hw2 & 0x7FF
                I1 = 1 - (J1 ^ S)
                I2 = 1 - (J2 ^ S)
                imm = (S << 24) | (I1 << 23) | (I2 << 22) | (imm10 << 12) | (imm11 << 1)
                if S:
                    imm -= 1 << 25
                print(f"  {hex(va)}: B.W -> {hex((va+4+imm)&0xffffffff)}")
                va += 4
                continue
            # BNE.W / BEQ.W etc (conditional B.W)
            if (hw & 0xF800) == 0xF000 and (hw2 & 0xD000) == 0x8000:
                S = (hw >> 10) & 1
                imm6 = hw & 0x3F
                # Actually: T3 conditional
                # F000 80xx style
                pass
            if (hw & 0xFBF0) == 0xF240:
                i = (hw >> 10) & 1
                imm4 = hw & 0xF
                imm3 = (hw2 >> 12) & 7
                rd = (hw2 >> 8) & 0xF
                imm8 = hw2 & 0xFF
                imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                print(f"  {hex(va)}: MOVW r{rd},#{hex(imm)}")
                va += 4
                continue
            if (hw & 0xFBF0) == 0xF2C0:
                i = (hw >> 10) & 1
                imm4 = hw & 0xF
                imm3 = (hw2 >> 12) & 7
                rd = (hw2 >> 8) & 0xF
                imm8 = hw2 & 0xFF
                imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                print(f"  {hex(va)}: MOVT r{rd},#{hex(imm)}")
                va += 4
                continue
            # BNE.W encoding: 11110 S cond imm6 | 10 J1 0 J2 imm11
            if (hw & 0xF800) == 0xF000 and (hw2 & 0xC000) == 0x8000 and (hw2 & 0x2000) == 0:
                S = (hw >> 10) & 1
                cond = (hw >> 6) & 0xF
                imm6 = hw & 0x3F
                J1 = (hw2 >> 13) & 1
                J2 = (hw2 >> 11) & 1
                imm11 = hw2 & 0x7FF
                imm = (S << 20) | (J2 << 19) | (J1 << 18) | (imm6 << 12) | (imm11 << 1)
                if S:
                    imm -= 1 << 21
                names = {0:"EQ",1:"NE",2:"CS",3:"CC",4:"MI",5:"PL",6:"VS",7:"VC",
                         8:"HI",9:"LS",10:"GE",11:"LT",12:"GT",13:"LE"}
                print(f"  {hex(va)}: B{names.get(cond,'??')}.W -> {hex((va+4+imm)&0xffffffff)}")
                va += 4
                continue
        # 16-bit
        if (hw & 0xFF00) == 0x2000:
            print(f"  {hex(va)}: MOVS r{(hw>>8)&7},#{hw&0xff}")
            va += 2
            continue
        if (hw & 0xF800) == 0x2800:
            print(f"  {hex(va)}: CMP r{(hw>>8)&7},#{hw&0xff}")
            va += 2
            continue
        if (hw & 0xF800) == 0x7800:
            print(f"  {hex(va)}: LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]")
            va += 2
            continue
        if (hw & 0xFF00) == 0xB100:
            rn = (hw >> 3) & 7
            imm5 = ((hw >> 2) & 0x1F) << 1  # approx
            # CBZ Rn, label: 1011 0x0i iiii iiii
            i = (hw >> 9) & 1
            imm5 = (hw >> 3) & 0x1F
            rn = hw & 7
            imm = (i << 6) | (imm5 << 1)
            print(f"  {hex(va)}: CBZ r{rn} -> {hex(va+4+imm)}")
            va += 2
            continue
        if (hw & 0xFF00) == 0xB900:
            i = (hw >> 9) & 1
            imm5 = (hw >> 3) & 0x1F
            rn = hw & 7
            imm = (i << 6) | (imm5 << 1)
            print(f"  {hex(va)}: CBNZ r{rn} -> {hex(va+4+imm)}")
            va += 2
            continue
        if (hw & 0xF000) == 0xD000 and ((hw >> 8) & 0xF) != 0xF:
            cond = (hw >> 8) & 0xF
            imm = hw & 0xFF
            if imm & 0x80:
                imm -= 0x100
            names = {0:"EQ",1:"NE",2:"CS",3:"CC",4:"MI",5:"PL",6:"VS",7:"VC",
                     8:"HI",9:"LS",10:"GE",11:"LT",12:"GT",13:"LE"}
            print(f"  {hex(va)}: B{names.get(cond,'??')} -> {hex(va+4+imm*2)}")
            va += 2
            continue
        if hw == 0xE000 or (hw & 0xF800) == 0xE000:
            imm = hw & 0x7FF
            if imm & 0x400:
                imm -= 0x800
            print(f"  {hex(va)}: B -> {hex(va+4+imm*2)}")
            va += 2
            continue
        print(f"  {hex(va)}: {hw:04x}")
        va += 2

# Entry gate + Present path already known; dump PIN-side branch destination
# From status-full: BNE.W at 0x14fb34e -> need target
# Encoding at 0x14fb34e: f040 80bc
hw = half(0x14FB34E)
hw2 = half(0x14FB350)
print(f"raw 0x14fb34e = {hw:04x} {hw2:04x}")
S = (hw >> 10) & 1
cond = (hw >> 6) & 0xF
imm6 = hw & 0x3F
J1 = (hw2 >> 13) & 1
J2 = (hw2 >> 11) & 1
imm11 = hw2 & 0x7FF
imm = (S << 20) | (J2 << 19) | (J1 << 18) | (imm6 << 12) | (imm11 << 1)
if S:
    imm -= 1 << 21
print(f"BNE.W after CMP#4 -> {hex((0x14FB34E+4+imm)&0xffffffff)} cond={cond}")

dump(0x14FB4CA, 0x14FB5E0, "STATUS app not in {1,4}")
dump(0x146AA40, 0x146AB20, "SET_APP#1 window")

# Who BLs into SET#1 function? Find function start by scanning back for PUSH
# Search BL targets into 0x146a900..0x146ab00
hits = []
for o in range(0, len(img) - 4, 2):
    va = VA + (o - MAIN)
    if va < 0x40010000 or va > 0x46000000:
        continue
    t = bl_target(va)
    if t and 0x146A900 <= t <= 0x146AABA:
        hits.append((va, t))
print("\nBL into SET#1 region:", [(hex(a), hex(b)) for a, b in hits[:30]], "n=", len(hits))
