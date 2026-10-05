#!/usr/bin/env python3
"""Find MOVW+MOVT xrefs to OEM encode strings; recover wire writes.

No live I/O. No invented bytes.
"""
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


def dump(o, n=48):
    return " ".join(f"{x:02x}" for x in img[o : o + n])


def decode_movw_movt_imm(w, w2):
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    return rd, imm


def find_movw_movt_va(target_va, band=None, limit=30):
    """Find MOVW low + nearby MOVT high assembling target_va."""
    lo16 = target_va & 0xFFFF
    hi16 = (target_va >> 16) & 0xFFFF
    lo, hi = (0, len(img) - 8) if band is None else band
    hits = []
    i = lo & ~1
    while i < hi and len(hits) < limit:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = decode_movw_movt_imm(w, w2)
            if imm == lo16:
                # look ahead for MOVT same rd within 16 bytes
                for j in range(i + 4, min(i + 20, hi), 2):
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = decode_movw_movt_imm(ww, ww2)
                        if rd2 == rd and imm2 == hi16:
                            hits.append((i, j, rd, target_va))
                            break
                    if (ww & 0xF800) >= 0xE800:
                        continue
            i += 4
        elif (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    return hits


def disasm_interesting(start, length=0x280):
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xF800) >= 0xE800:
            if (w & 0xFBF0) == 0xF240:
                rd, imm = decode_movw_movt_imm(w, w2)
                out.append((i, f"MOVW r{rd},#{hex(imm)}"))
            elif (w & 0xFBF0) == 0xF2C0:
                rd, imm = decode_movw_movt_imm(w, w2)
                out.append((i, f"MOVT r{rd},#{hex(imm)}"))
            elif (w & 0xFFF0) == 0xF890:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"LDRB.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8B0:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"LDRH.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8D0:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"LDR.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF880:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"STRB.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8A0:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"STRH.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8C0:
                rt, rn, imm = (w2 >> 12) & 0xF, w & 0xF, w2 & 0xFFF
                if imm <= 0x100:
                    out.append((i, f"STR.W r{rt},[r{rn},#{imm}]"))
            # ADDW / SUBW
            elif (w & 0xFBF0) == 0xF200:
                rd = (w2 >> 8) & 0xF
                rn = w & 0xF
                i_bit = (w >> 10) & 1
                imm3 = (w2 >> 12) & 7
                imm8 = w2 & 0xFF
                imm = (i_bit << 11) | (imm3 << 8) | imm8
                out.append((i, f"ADDW r{rd},r{rn},#{imm}"))
            i += 4
        else:
            if (w & 0xF800) == 0x2000:
                out.append((i, f"MOVS r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x2800:
                out.append((i, f"CMP r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x7800:
                imm = (w >> 6) & 0x1F
                out.append((i, f"LDRB r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x7000:
                imm = (w >> 6) & 0x1F
                out.append((i, f"STRB r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x8800:
                imm = ((w >> 6) & 0x1F) << 1
                out.append((i, f"LDRH r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x8000:
                imm = ((w >> 6) & 0x1F) << 1
                out.append((i, f"STRH r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x6800:
                imm = ((w >> 6) & 0x1F) << 2
                out.append((i, f"LDR r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x6000:
                imm = ((w >> 6) & 0x1F) << 2
                out.append((i, f"STR r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
                out.append((i, f"PUSH {hex(w)}"))
            elif w == 0xE92D:
                out.append((i, f"PUSH.W {hex(w2)}"))
            i += 2
    return out


def find_prolog(addr, back=0x200):
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
            return j
        if w == 0xE92D:
            return j
    return max(0, addr - 0x40)


# Target strings
targets = {
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "alloc": b"[OEM][IPC] Unable to allocate size %u",
    "decode": b"[OEM][IPC] Unable to decode",
    "invalid": b"[OEM][IPC] Invalid message",
    "sit_send": b"[OEM][SIT] Sending data length %u to channel %u",
}

# Also search truncated "Unable to encode IPC message" alone at 0x6dfcf3
print("=== MOVW+MOVT xrefs (OEM band + whole) ===")
for name, needle in targets.items():
    o = img.find(needle)
    if o < 0:
        print(f"{name}: MISSING")
        continue
    va = va_of(o)
    print(f"\n{name}: off={hex(o)} va={hex(va)}")
    # Prefer OEM code band first
    hits = find_movw_movt_va(va, band=(0x6c0000, 0x720000), limit=20)
    if not hits:
        hits = find_movw_movt_va(va, band=(0x600000, 0x800000), limit=20)
    if not hits:
        hits = find_movw_movt_va(va, limit=10)
    print(f"  movw/movt hits={len(hits)} {[hex(h[0]) for h in hits[:8]]}")
    for movw_off, movt_off, rd, _ in hits[:3]:
        prolog = find_prolog(movw_off)
        print(f"  --- fn~{hex(prolog)} va={hex(va_of(prolog))} ref@{hex(movw_off)} r{rd} ---")
        for off, txt in disasm_interesting(prolog, 0x300):
            print(f"    {hex(off)}: {txt}")

# Pointer-table refs: who loads from lit word containing VA?
print("\n=== pointer-table consumers (LDR from nearby code using table) ===")
for name, needle in list(targets.items())[:5]:
    o = img.find(needle)
    va = va_of(o)
    # find 4-byte stores of VA
    for lit in [] if o < 0 else [i for i in range(max(0, o - 0x200), min(len(img) - 4, o + 0x200), 4) if u32(i) == va]:
        print(f"{name} ptrword@{hex(lit)}")
        # search for ADDW/ADR patterns hard; instead dump ±0x40 ascii/code context
        print("  neigh:", dump(lit - 16, 48))

# Catalog body-size correlation: dump several SIM_* with flags
print("\n=== catalog stride-28 from aligned SIM_INIT ===")
# Find true start: scan back from 0x6de740 for consistent meta=0x10104
base = 0x6de740
while base > 0x6de000:
    prev = base - 28
    meta = u32(prev + 8)
    msgid = u16(prev + 2)
    if meta == 0x10104 and 0x2F00 <= msgid <= 0x2FFF:
        base = prev
    else:
        break
print(f"catalog walk start@{hex(base)}")
o = base
for i in range(25):
    flags = u16(o)
    msgid = u16(o + 2)
    nameptr = u32(o + 4)
    meta = u32(o + 8)
    rsp = u32(o + 12)
    w4 = u32(o + 16)
    w5 = u32(o + 20)
    w6 = u32(o + 24)
    name = cstr(off_of(nameptr), 40) if VA0 <= nameptr < VA0 + 0x8000000 else None
    mark = " <<<" if msgid in (0x2F50, 0x2F52, 0x2F57, 0x2F58) else ""
    print(
        f"  @{hex(o)} flags={flags:3d} id={hex(msgid)} meta={hex(meta)} rsp={hex(rsp)} w4={hex(w4)} w5={hex(w5)} w6={hex(w6)} name={name}{mark}"
    )
    o += 28

# Cross-check: does encode path use catalog flags as encode size?
# Search for LDRH from catalog+0 (flags) near OEM band with CMP
print("\n=== hunt encode size from catalog flags (LDRH imm#0 / ADD #0) near not_req/encode ===")
# Disassemble wide window around first pointer table near encode strings
# The strings sit in a data island; code is typically BEFORE the strings.
print("code window before encode strings @0x6df000..0x6dfce8")
for off, txt in disasm_interesting(0x6df000, 0xCE0):
    if any(k in txt for k in ("STR", "LDR", "MOVW", "MOVT", "CMP", "MOVS", "PUSH", "ADDW")):
        # filter noise: only small immediates or mov of interesting constants
        if "MOVW" in txt or "MOVT" in txt or "PUSH" in txt or "CMP" in txt or "MOVS" in txt:
            print(f"  {hex(off)}: {txt}")
        elif any(f"#{n}" in txt for n in (0, 1, 2, 4, 6, 8, 9, 10, 12, 14, 16, 18, 19, 20, 24, 28)):
            print(f"  {hex(off)}: {txt}")

print("\n=== code AFTER not_req string toward sit_send ===")
for off, txt in disasm_interesting(0x6dff80, 0x200):
    if "MOVW" in txt or "MOVT" in txt or "STR" in txt or "LDR" in txt or "CMP" in txt or "PUSH" in txt:
        print(f"  {hex(off)}: {txt}")

# Look for file path string used in asserts: oem_ipc_message_utils.c often paired with line
utils = img.find(b"oem_ipc_message_utils.c")
print(f"\nutils str@{hex(utils)} va={hex(va_of(utils))}")
# find MOVW/MOVT to utils
if utils >= 0:
    hits = find_movw_movt_va(va_of(utils), limit=20)
    print(f"utils movw hits={len(hits)}")
    for movw_off, movt_off, rd, _ in hits[:5]:
        prolog = find_prolog(movw_off)
        print(f"  fn~{hex(prolog)} ref@{hex(movw_off)}")
        for off, txt in disasm_interesting(prolog, 0x100):
            print(f"    {hex(off)}: {txt}")

print("DONE")
