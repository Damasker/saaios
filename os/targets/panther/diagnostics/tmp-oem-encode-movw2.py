#!/usr/bin/env python3
"""MOVW+MOVT xrefs in OEM band only; catalog walk; wire-field hunt."""
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


def find_movw_movt_va(target_va, lo, hi, limit=20):
    lo16 = target_va & 0xFFFF
    hi16 = (target_va >> 16) & 0xFFFF
    hits = []
    i = lo & ~1
    while i < hi - 8 and len(hits) < limit:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            if imm == lo16:
                j = i + 4
                while j < min(i + 24, hi - 4):
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd and imm2 == hi16:
                            hits.append((i, j, rd))
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
    return hits


def find_prolog(addr, back=0x180):
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400 or w == 0xE92D:
            return j
    return max(0, addr - 0x20)


def disasm(start, length=0x200):
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
            elif (w & 0xF800) == 0x7800:
                out.append((i, f"LDRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x7000:
                out.append((i, f"STRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x8800:
                out.append((i, f"LDRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xF800) == 0x8000:
                out.append((i, f"STRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xF800) == 0x6800:
                out.append((i, f"LDR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xF800) == 0x6000:
                out.append((i, f"STR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
                out.append((i, f"PUSH {hex(w)}"))
            i += 2
    return out


# Pre-index all MOVW in OEM-ish code bands once
BANDS = [(0x680000, 0x720000), (0x3900000, 0x3920000)]  # OEM + SIM_INIT handler area


def index_movw(bands):
    idx = {}  # imm16 -> list of (off, rd)
    for lo, hi in bands:
        i = lo & ~1
        while i < hi - 4:
            w = u16(i)
            w2 = u16(i + 2)
            if (w & 0xFBF0) == 0xF240:
                rd, imm = dec_imm(w, w2)
                idx.setdefault(imm, []).append((i, rd))
                i += 4
            elif (w & 0xF800) >= 0xE800:
                i += 4
            else:
                i += 2
    return idx


print("indexing MOVW...")
movw_idx = index_movw(BANDS)
print(f"unique imm16={len(movw_idx)}")


def hits_for_va(va):
    lo16 = va & 0xFFFF
    hi16 = (va >> 16) & 0xFFFF
    out = []
    for off, rd in movw_idx.get(lo16, []):
        j = off + 4
        while j < off + 24:
            ww = u16(j)
            ww2 = u16(j + 2)
            if (ww & 0xFBF0) == 0xF2C0:
                rd2, imm2 = dec_imm(ww, ww2)
                if rd2 == rd and imm2 == hi16:
                    out.append((off, j, rd))
                    break
            if (ww & 0xF800) >= 0xE800:
                j += 4
            else:
                j += 2
    return out


targets = {
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "alloc": b"[OEM][IPC] Unable to allocate size %u",
    "decode_fail": b"[OEM][IPC] Unable to decode",
    "sit_send": b"[OEM][SIT] Sending data length %u to channel %u",
    "sit_recv": b"[OEM][SIT] Received packet from channel %u, size %u",
    "host_not_ready": b"[OEM][SIT] Host interface is not ready",
}

print("\n=== string MOVW+MOVT xrefs ===")
for name, needle in targets.items():
    o = img.find(needle)
    if o < 0:
        print(f"{name}: MISSING")
        continue
    va = va_of(o)
    hits = hits_for_va(va)
    print(f"{name}: off={hex(o)} va={hex(va)} hits={len(hits)} {[hex(h[0]) for h in hits]}")
    for movw_off, movt_off, rd in hits[:2]:
        prolog = find_prolog(movw_off)
        print(f"  fn~{hex(prolog)} va={hex(va_of(prolog))}")
        for off, txt in disasm(prolog, 0x2C0):
            print(f"    {hex(off)}: {txt}")

# Also try loading string via pointer table: scan words near strings that equal VA,
# then find ADR/LDR to those table slots — hard. Instead: scan for BL targets.

# Catalog walk
print("\n=== catalog stride-28 ===")
base = 0x6de740
while True:
    prev = base - 28
    if u32(prev + 8) == 0x10104 and 0x2F00 <= u16(prev + 2) <= 0x2FFF:
        base = prev
    else:
        break
print(f"start@{hex(base)}")
o = base
for _ in range(30):
    flags, msgid = u16(o), u16(o + 2)
    nameptr, meta, rsp = u32(o + 4), u32(o + 8), u32(o + 12)
    w4, w5, w6 = u32(o + 16), u32(o + 20), u32(o + 24)
    name = cstr(off_of(nameptr), 48) if VA0 <= nameptr < VA0 + 0x8000000 else None
    mark = " <<<" if msgid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x2FA1) else ""
    print(
        f"  @{hex(o)} body={flags:3d} id={hex(msgid)} meta={hex(meta)} rsp={hex(rsp)} w4={hex(w4)} w5={hex(w5)} w6={hex(w6)} {name}{mark}"
    )
    if meta != 0x10104 and _ > 3:
        break
    o += 28

# Decode path: not_req checks message type — find object field offsets from prior note (#9/#18/#19)
print("\n=== disasm around lit words at 0x6dfdxx (encode island) ===")
# Even if not LDR-pc, dump raw around encode strings and look for Thumb code BEFORE data
# Find last PUSH before 0x6dfc00
code_end = 0x6dfc00
# walk back for dense code
for start in (0x6db000, 0x6dc000, 0x6dd000, 0x6de000, 0x6df000):
    # count PUSH in window
    pushes = 0
    i = start
    while i < start + 0x1000:
        w = u16(i)
        if (w & 0xFF00) == 0xB500:
            pushes += 1
        if (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    print(f"  PUSH count in {hex(start)}..+0x1000 = {pushes}")

# Broader band for movw to encode_fail VA
print("\n=== broaden MOVW index 0x600000..0x800000 for encode strings only ===")
enc_vas = {}
for name, needle in targets.items():
    o = img.find(needle)
    if o >= 0:
        enc_vas[name] = va_of(o)

# single pass
i = 0x600000
found = {k: [] for k in enc_vas}
while i < 0x800000 - 8:
    w = u16(i)
    w2 = u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        rd, imm = dec_imm(w, w2)
        for name, va in enc_vas.items():
            if imm == (va & 0xFFFF):
                j = i + 4
                while j < i + 24:
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd and imm2 == ((va >> 16) & 0xFFFF):
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

for name, hits in found.items():
    print(f"  {name}: {len(hits)} {[hex(h) for h in hits[:8]]}")
    for h in hits[:1]:
        prolog = find_prolog(h)
        print(f"    fn~{hex(prolog)}")
        for off, txt in disasm(prolog, 0x2C0):
            print(f"      {hex(off)}: {txt}")

# Check if strings are only referenced from pointer tables (rodata), not code
print("\n=== pointer words equal to encode_fail VA ===")
efa = enc_vas.get("encode_fail")
if efa:
    t = struct.pack("<I", efa)
    pos = 0
    n = 0
    while n < 20:
        i = img.find(t, pos)
        if i < 0:
            break
        print(f"  ptr@{hex(i)} neigh_ascii={cstr(i-32, 80)!r}")
        pos = i + 4
        n += 1

print("DONE")
