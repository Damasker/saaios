#!/usr/bin/env python3
"""Find code refs to OEM catalog/ptr-table bases. No nested zip."""
from __future__ import annotations

import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA0 = 0x40010000
MAIN = 0x16C10


def va_of(o):
    return VA0 + (o - MAIN)


def off_of(va):
    return (va - VA0) + MAIN


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def cstr(o, n=80):
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def dec_imm(w, w2):
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    return rd, (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8


def find_prolog(addr, back=0x200):
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400 or w == 0xE92D:
            return j
    return max(0, addr - 0x20)


def disasm(start, length=0x280):
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xF800) >= 0xE800:
            if (w & 0xFBF0) == 0xF240:
                rd, imm = dec_imm(w, w2)
                out.append((i, f"MOVW r{rd},#{hex(imm)}"))
            elif (w & 0xFBF0) == 0xF2C0:
                rd, imm = dec_imm(w, w2)
                out.append((i, f"MOVT r{rd},#{hex(imm)}"))
            elif (w & 0xFFF0) in (0xF890, 0xF8B0, 0xF8D0, 0xF880, 0xF8A0, 0xF8C0):
                op = {
                    0xF890: "LDRB.W",
                    0xF8B0: "LDRH.W",
                    0xF8D0: "LDR.W",
                    0xF880: "STRB.W",
                    0xF8A0: "STRH.W",
                    0xF8C0: "STR.W",
                }[w & 0xFFF0]
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0xC0:
                    out.append((i, f"{op} r{rt},[r{rn},#{imm}]"))
            i += 4
        else:
            if (w & 0xF800) == 0x2000:
                out.append((i, f"MOVS r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x2800:
                out.append((i, f"CMP r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) in (0x7800, 0x7000, 0x8800, 0x8000, 0x6800, 0x6000):
                kind = {
                    0x7800: ("LDRB", 1),
                    0x7000: ("STRB", 1),
                    0x8800: ("LDRH", 2),
                    0x8000: ("STRH", 2),
                    0x6800: ("LDR", 4),
                    0x6000: ("STR", 4),
                }[w & 0xF800]
                imm = ((w >> 6) & 0x1F) * kind[1]
                out.append((i, f"{kind[0]} r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
                out.append((i, f"PUSH {hex(w)}"))
            i += 2
    return out


def scan_vas(vas):
    want = {}
    for name, va in vas.items():
        want.setdefault((va & 0xFFFF, (va >> 16) & 0xFFFF), []).append(name)
    found = {n: [] for n in vas}
    i = 0
    hi = len(img) - 8
    while i < hi:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            cands = [k for k in want if k[0] == imm]
            if cands:
                j = i + 4
                while j < i + 28 and j < hi:
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd:
                            for lo16, hi16 in cands:
                                if imm2 == hi16:
                                    for name in want[(lo16, hi16)]:
                                        found[name].append(i)
                            break
                    if (ww & 0xF800) >= 0xE800:
                        j += 4
                    else:
                        j += 2
            i += 4
        elif (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    return found


utils_o = img.find(b"oem_ipc_message_utils.c")
bases = {
    "catalog_2f50": va_of(0x6DE740),
    "catalog_start": va_of(0x6DE708),
    "ptr_encode_fail": va_of(0x6DFD58),
    "ptr_not_req": va_of(0x6DFF00),
    "ptr_table_base": va_of(0x6DFC0C),
    "str_encode_fail": va_of(0x6DFCE8),
    "str_utils": va_of(utils_o) if utils_o >= 0 else 0,
}
print("=== target VAs ===")
for k, v in bases.items():
    print(f"  {k}={hex(v)}")

print("\n=== MOVW+MOVT to bases ===")
found = scan_vas({k: v for k, v in bases.items() if v})
for k, hits in found.items():
    print(f"{k}: {len(hits)} {[hex(h) for h in hits[:10]]}")
    for h in hits[:2]:
        prolog = find_prolog(h)
        print(f"  fn~{hex(prolog)} va={hex(va_of(prolog))}")
        for off, txt in disasm(prolog, 0x2C0):
            print(f"    {hex(off)}: {txt}")

print("\n=== absolute ptr words == catalog entry VA ===")
for name, off in [("2f50", 0x6DE740), ("2f52", 0x6DE874), ("start", 0x6DE708)]:
    va = va_of(off)
    t = struct.pack("<I", va)
    pos = 0
    hits = []
    while len(hits) < 20:
        i = img.find(t, pos)
        if i < 0:
            break
        hits.append(i)
        pos = i + 4
    print(f"{name} va={hex(va)} n={len(hits)} ptrsites={[hex(h) for h in hits]}")

print("\n=== catalog words as possible code/data ptrs ===")
for off, label in [(0x6DE740, "INIT"), (0x6DE874, "VERIFYPIN"), (0x6DE724, "INFO")]:
    words = [u32(off + 4 + 4 * k) for k in range(6)]
    flags, msgid = u16(off), u16(off + 2)
    print(f"{label} @{hex(off)} body={flags} id={hex(msgid)} words={[hex(w) for w in words]}")
    for w in words:
        if 0x40010000 <= w <= 0x45000000:
            o = off_of(w)
            if 0 <= o < len(img) - 2:
                h = u16(o & ~1)
                print(
                    f"  -> {hex(w)} off={hex(o)} half={hex(h)} "
                    f"pushish={(h & 0xFF00) == 0xB500 or h == 0xE92D} ascii={cstr(o, 48)!r}"
                )

# Decode path object fields: scan for CMP #1 / REQUEST type near SIM handler band
# Prior: LDRB #9/#18/#19 near not_req — those were false (data bank).
# Search USIM/OEM code for msgid 0x2f50 load then encode-size use.
print("\n=== bare MOVW #0x2f50 then STRH within 64B (whole image, sample) ===")
count = 0
i = 0
while i < len(img) - 8 and count < 40:
    w = u16(i)
    w2 = u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        rd, imm = dec_imm(w, w2)
        if imm == 0x2F50:
            # skip if MOVT same rd follows (pointer assemble)
            nxt = u16(i + 4)
            nxt2 = u16(i + 6)
            is_movt = (nxt & 0xFBF0) == 0xF2C0 and ((nxt2 >> 8) & 0xF) == rd
            if not is_movt:
                # look for STRH in next 64 bytes
                strs = []
                j = i + 4
                while j < i + 64:
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xF800) == 0x8000:
                        strs.append((j, f"STRH r{ww&7},[r{(ww>>3)&7},#{((ww>>6)&0x1F)<<1}]"))
                    if (ww & 0xFFF0) == 0xF8A0:
                        strs.append(
                            (
                                j,
                                f"STRH.W r{(ww2>>12)&0xF},[r{ww&0xF},#{ww2&0xFFF}]",
                            )
                        )
                    if (ww & 0xF800) >= 0xE800:
                        j += 4
                    else:
                        j += 2
                if strs:
                    count += 1
                    print(f"  @{hex(i)} MOVW r{rd},#0x2f50 STRHs={strs[:4]}")
        i += 4
    elif (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2
print(f"reported={count}")
print("DONE")
