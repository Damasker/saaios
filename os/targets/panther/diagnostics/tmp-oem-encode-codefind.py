#!/usr/bin/env python3
"""Locate real oem_ipc encode code via utils.c path + ptr-table consumers.

No live I/O.
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


def cstr(o, n=100):
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


def disasm(start, length=0x240):
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


def scan_movw_movt_for_vas(vas: dict, lo: int, hi: int):
    """Single pass: find MOVW+MOVT assembling any of vas values."""
    want = {}
    for name, va in vas.items():
        want.setdefault((va & 0xFFFF, (va >> 16) & 0xFFFF), []).append((name, va))
    found = {name: [] for name in vas}
    i = lo & ~1
    while i < hi - 8:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            # check if lo16 matches any
            candidates = [k for k in want if k[0] == imm]
            if candidates:
                j = i + 4
                while j < i + 24:
                    ww = u16(j)
                    ww2 = u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd:
                            for lo16, hi16 in candidates:
                                if imm2 == hi16:
                                    for name, va in want[(lo16, hi16)]:
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


# Collect key VAs
vas = {}
for name, needle in {
    "utils_c": b"oem_ipc_message_utils.c",
    "disp_c": b"oem_ipc_message_dispatcher.c",
    "server_c": b"ipc_message_server",
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "sit_send": b"[OEM][SIT] Sending data length %u to channel %u",
}.items():
    o = img.find(needle)
    print(f"str {name}: off={hex(o) if o>=0 else -1} va={hex(va_of(o)) if o>=0 else -1} txt={cstr(o,70) if o>=0 else None}")
    if o >= 0:
        vas[name] = va_of(o)

# Full-image scan is ~98MB * check — doable in one pass ~few seconds
print("\n=== full-image MOVW+MOVT scan for key VAs ===")
found = scan_movw_movt_for_vas(vas, 0, len(img) - 8)
for name, hits in found.items():
    print(f"{name}: {len(hits)} {[hex(h) for h in hits[:12]]}")
    for h in hits[:2]:
        prolog = find_prolog(h)
        print(f"  fn~{hex(prolog)} va={hex(va_of(prolog))} ref@{hex(h)}")
        for off, txt in disasm(prolog, 0x2C0):
            print(f"    {hex(off)}: {txt}")

# Pointer-table dump around encode_fail
print("\n=== pointer table around encode strings ===")
# dump u32 words from 0x6dfc00..0x6e0200 that look like VAs
for o in range(0x6dfc00, 0x6e0280, 4):
    w = u32(o)
    if VA0 <= w < VA0 + 0x2000000:
        s = cstr(off_of(w), 60)
        if s and ("OEM" in s or "encode" in s or "IPC" in s or "SIT" in s or "Request" in s):
            print(f"  @{hex(o)} -> {hex(w)} {s!r}")

# Find LDR.W [pc] that land on those pointer words — Thumb-2 literal pools can be far
print("\n=== LDR.W pc-rel to ptr@0x6dfd58 (encode_fail ptr) ===")
target_lits = []
efa = vas.get("encode_fail")
if efa:
    t = struct.pack("<I", efa)
    pos = 0
    while True:
        i = img.find(t, pos)
        if i < 0:
            break
        target_lits.append(i)
        pos = i + 4
print("ptr sites:", [hex(x) for x in target_lits])

# Scan code regions that have many PUSH (real code) for LDR.W hitting these lits
# Regions: sample every potential LDR.W F85F / F8DF
def find_ldr_w_pc_to(lit_off, code_lo, code_hi):
    hits = []
    i = code_lo & ~1
    while i < code_hi - 4:
        w = u16(i)
        w2 = u16(i + 2)
        # LDR.W Rt,[PC, #+/-imm] : F85F / F8DF variants
        if (w & 0xFF7F) == 0xF85F:
            rt = (w2 >> 12) & 0xF
            imm = w2 & 0xFFF
            u = (w >> 7) & 1
            base = (i + 4) & ~3
            lit = base + imm if u else base - imm
            if lit == lit_off or abs(lit - lit_off) <= 2:
                hits.append((i, rt, lit))
        if (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    return hits


# Heuristic code bands: where PUSH density is high
print("scanning LDR.W to encode ptrs in 0x100000..0x2000000 (chunked)...")
for lit in target_lits[:3]:
    all_hits = []
    for lo in range(0x100000, 0x2000000, 0x200000):
        hi = min(lo + 0x200000, 0x2000000)
        all_hits.extend(find_ldr_w_pc_to(lit, lo, hi))
    print(f"  lit@{hex(lit)} LDR.W hits={len(all_hits)} {[hex(h[0]) for h in all_hits[:8]]}")
    for h in all_hits[:2]:
        prolog = find_prolog(h[0])
        print(f"    fn~{hex(prolog)}")
        for off, txt in disasm(prolog, 0x200):
            print(f"      {hex(off)}: {txt}")

# nanopb / encode size: look for SIM_INIT_REQ near descriptor bytes
print("\n=== bytes before SIM_INIT_REQ name string ===")
name_o = off_of(0x410305f3) if False else img.find(b"SIM_INIT_REQ\x00")
print(f"name@{hex(name_o)}")
if name_o > 0:
    print("before:", " ".join(f"{x:02x}" for x in img[name_o - 32 : name_o + 16]))

# meta 0x10104 bit decode attempt
print("\n=== meta=0x10104 across SIM catalog (bitfields) ===")
# already know it's common; print binary
print(f"0x10104 = {bin(0x10104)} = bits set {[i for i in range(32) if (0x10104>>i)&1]}")

print("DONE")
