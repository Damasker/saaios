#!/usr/bin/env python3
"""Deep RE: MAIN oem_ipc encode path near Unable-to-encode / not-REQUEST.

Recover app header + body for SIM_INIT_REQ 0x2f50 (cross-check 0x2f52).
No live I/O. No invented wire bytes in verdict.
"""
from __future__ import annotations

import struct
from pathlib import Path

MAIN_PATHS = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"),
]
img = None
for p in MAIN_PATHS:
    if p.exists():
        img = p.read_bytes()
        print(f"MAIN={p} size={len(img)}")
        break
if img is None:
    raise SystemExit("MAIN missing")

VA0 = 0x40010000
MAIN = 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN


def u8(o: int) -> int:
    return img[o]


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 100):
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


def dump(o: int, n: int = 64) -> str:
    return " ".join(f"{x:02x}" for x in img[o : o + n])


def find_all(hay: bytes, needle: bytes, limit: int = 40):
    out = []
    pos = 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def find_litpool_refs(target_va: int, band=None, limit: int = 40):
    hits = []
    lo, hi = (0, len(img) - 4) if band is None else band
    t = struct.pack("<I", target_va & 0xFFFFFFFF)
    pos = lo
    while len(hits) < limit:
        i = img.find(t, pos, hi)
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    return hits


def ldr_pc_hits_for_lit(lit_off: int, back: int = 0x280):
    hits = []
    for i in range(max(0, lit_off - back), lit_off, 2):
        w = u16(i)
        if (w & 0xF800) == 0x4800:  # LDR Rt,[pc,#imm] T1
            rt = (w >> 8) & 7
            imm = (w & 0xFF) << 2
            lit = ((i + 4) & ~2) + imm
            if lit == lit_off or abs(lit - lit_off) <= 2:
                hits.append((i, rt, lit))
        # LDR.W Rt,[pc,#imm] T2: F8DF
        if (w & 0xFF7F) == 0xF85F and i + 4 <= len(img):
            w2 = u16(i + 2)
            rt = (w2 >> 12) & 0xF
            imm = w2 & 0xFFF
            # U bit in w
            u = (w >> 7) & 1
            base = (i + 4) & ~3
            lit = base + imm if u else base - imm
            if lit == lit_off or abs(lit - lit_off) <= 2:
                hits.append((i, rt, lit))
    return hits


def disasm_window(start: int, length: int = 0x200):
    """Emit MOVW/MOVT/LDR-pc + interesting LDR*/STR* imm accesses."""
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        if (w & 0xF800) >= 0xE800:
            if (w & 0xFBF0) == 0xF240:
                i_bit = (w >> 10) & 1
                imm4 = w & 0xF
                imm3 = (w2 >> 12) & 7
                rd = (w2 >> 8) & 0xF
                imm8 = w2 & 0xFF
                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                out.append((i, f"MOVW r{rd},#{hex(imm)}"))
            elif (w & 0xFBF0) == 0xF2C0:
                i_bit = (w >> 10) & 1
                imm4 = w & 0xF
                imm3 = (w2 >> 12) & 7
                rd = (w2 >> 8) & 0xF
                imm8 = w2 & 0xFF
                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                out.append((i, f"MOVT r{rd},#{hex(imm)}"))
            elif (w & 0xFFF0) == 0xF890:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"LDRB.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8B0:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"LDRH.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8D0:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"LDR.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF880:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"STRB.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8A0:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"STRH.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF8C0:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFFF
                if imm <= 0x80:
                    out.append((i, f"STR.W r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFF7F) == 0xF85F:
                rt = (w2 >> 12) & 0xF
                imm = w2 & 0xFFF
                u = (w >> 7) & 1
                base = (i + 4) & ~3
                lit = base + imm if u else base - imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                s = None
                if VA0 <= val < VA0 + 0x8000000:
                    s = cstr(off_of(val), 60)
                out.append((i, f"LDR.W r{rt},[pc] lit@{hex(lit)}={hex(val)} '{s}'"))
            i += 4
        else:
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                s = None
                if VA0 <= val < VA0 + 0x8000000:
                    s = cstr(off_of(val), 60)
                out.append((i, f"LDR r{rt},[pc+{imm}] lit@{hex(lit)}={hex(val)} '{s}'"))
            elif (w & 0xF800) == 0x7800:
                imm = (w >> 6) & 0x1F
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"LDRB r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0x7000:
                imm = (w >> 6) & 0x1F
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"STRB r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0x8800:
                imm = ((w >> 6) & 0x1F) << 1
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"LDRH r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0x8000:
                imm = ((w >> 6) & 0x1F) << 1
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"STRH r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0x6800:
                imm = ((w >> 6) & 0x1F) << 2
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"LDR r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0x6000:
                imm = ((w >> 6) & 0x1F) << 2
                rt = w & 7
                rn = (w >> 3) & 7
                out.append((i, f"STR r{rt},[r{rn},#{imm}]"))
            # MOVS Rd,#imm8
            elif (w & 0xF800) == 0x2000:
                rd = (w >> 8) & 7
                imm = w & 0xFF
                out.append((i, f"MOVS r{rd},#{imm}"))
            # CMP Rn,#imm8
            elif (w & 0xF800) == 0x2800:
                rn = (w >> 8) & 7
                imm = w & 0xFF
                out.append((i, f"CMP r{rn},#{imm}"))
            i += 2
    return out


def parse_cat28(o: int):
    # Prior: flags:u16 msgid:u16 ... meta ... name
    flags = u16(o)
    msgid = u16(o + 2)
    words = [u32(o + 4 + 4 * k) for k in range(6)]
    name = None
    name_off = None
    for w in words:
        if VA0 <= w < VA0 + 0x8000000:
            s = cstr(off_of(w), 48)
            if s and ("SIM_" in s or "REQ" in s or "IND" in s or "CNF" in s or "RSP" in s):
                name = s
                name_off = off_of(w)
                break
            if s and s[:3].isalpha() and "_" in s:
                name = s
                name_off = off_of(w)
                break
    return {
        "off": hex(o),
        "va": hex(va_of(o)),
        "flags": flags,
        "msgid": hex(msgid),
        "words": [hex(x) for x in words],
        "name": name,
        "name_off": hex(name_off) if name_off is not None else None,
        "raw": dump(o, 28),
    }


print("=== key strings ===")
strings = {
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "utils": b"oem_ipc_message_utils.c",
    "disp": b"oem_ipc_message_dispatcher.c",
    "server": b"ipc_message_server",
    "invalid": b"[OEM][IPC] Invalid message",
    "alloc": b"[OEM][IPC] Unable to allocate size %u",
    "decode": b"[OEM][IPC] Unable to decode",
    "sit_send": b"[OEM][SIT] Sending data length %u to channel %u",
    "kMessageId": b"kMessageId",
    "Request": b"is not a REQUEST",
}
str_meta = {}
for k, n in strings.items():
    o = img.find(n)
    if o < 0:
        print(f"{k}: MISSING")
        continue
    va = va_of(o)
    lits = find_litpool_refs(va, limit=12)
    str_meta[k] = {"off": o, "va": va, "lits": lits}
    print(f"{k}: off={hex(o)} va={hex(va)} litrefs={[hex(x) for x in lits[:6]]}")

# Focus on encode_fail / not_req as hinted
for key in ("encode_fail", "not_req", "enc_size", "msgid_nf", "alloc"):
    meta = str_meta.get(key)
    if not meta:
        continue
    print(f"\n=== xrefs / disasm around {key} ===")
    for lit in meta["lits"][:4]:
        ldrs = ldr_pc_hits_for_lit(lit)
        print(f"  lit@{hex(lit)} LDR hits={[ (hex(a), f'r{b}') for a,b,_ in ldrs[:8] ]}")
        if not ldrs:
            # scan nearby for any LDR that lands near
            continue
        fn = ldrs[0][0]
        # walk back to likely function prologue (PUSH)
        prolog = fn
        for j in range(fn, max(0, fn - 0x180), -2):
            w = u16(j)
            # PUSH {..} T1 0xB4xx or T2 0xE92D
            if (w & 0xFE00) == 0xB400 or w == 0xE92D or (w & 0xFF00) == 0xB500:
                prolog = j
                break
        print(f"  approx_fn@{hex(prolog)} va={hex(va_of(prolog))} (from LDR@{hex(fn)})")
        for off, txt in disasm_window(prolog, 0x220):
            print(f"    {hex(off)}: {txt}")

# Catalog entries for 0x2f50 / 0x2f52
print("\n=== catalog 0x2f50 / 0x2f52 ===")
for msgid in (0x2f50, 0x2f52, 0x2f58, 0x2fa1):
    pos = 0
    found = []
    while len(found) < 8:
        i = img.find(struct.pack("<H", msgid), pos)
        if i < 0:
            break
        # msgid at +2 of entry
        if i >= 2:
            cand = i - 2
            # crude: flags small
            flags = u16(cand)
            if flags in (0, 1, 2, 4, 6, 8, 10, 12, 16, 20, 24, 28, 32, 36, 40, 48, 64):
                found.append(cand)
        pos = i + 1
    print(f"msgid={hex(msgid)} candidates={len(found)}")
    for o in found[:4]:
        print(" ", parse_cat28(o))

# Prior known catalog @0x6de740
print("\n=== forced catalog @0x6de700.. ===")
for o in range(0x6de700, 0x6de800, 28):
    print(" ", parse_cat28(o))

# Hunt encode: look for STRH/STRB building a buffer near MOVW #0x2f50 in OEM band
print("\n=== MOVW #0x2f50 bare (no MOVT follow) in OEM band 0x6d0000..0x6f0000 ===")
band = range(0x6d0000, min(0x6f0000, len(img) - 4), 2)
movw_hits = []
i = 0x6d0000
while i < 0x6f0000 and i + 4 < len(img):
    w = u16(i)
    w2 = u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        i_bit = (w >> 10) & 1
        imm4 = w & 0xF
        imm3 = (w2 >> 12) & 7
        rd = (w2 >> 8) & 0xF
        imm8 = w2 & 0xFF
        imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
        if imm == 0x2F50:
            # check next isn't MOVT same rd with high half making pointer
            nxt = u16(i + 4)
            nxt2 = u16(i + 6)
            is_movt = (nxt & 0xFBF0) == 0xF2C0
            movt_rd = (nxt2 >> 8) & 0xF if is_movt else None
            movt_imm = None
            if is_movt:
                i_bit2 = (nxt >> 10) & 1
                imm4b = nxt & 0xF
                imm3b = (nxt2 >> 12) & 7
                imm8b = nxt2 & 0xFF
                movt_imm = (i_bit2 << 11) | (imm4b << 12) | (imm3b << 8) | imm8b
            bare = not (is_movt and movt_rd == rd)
            movw_hits.append((i, rd, bare, movt_imm))
            print(f"  @{hex(i)} MOVW r{rd},#0x2f50 bare={bare} movt_imm={hex(movt_imm) if movt_imm is not None else None}")
        i += 4
    elif (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2

# Cross-check 0x2f52 movw in same band
print("\n=== MOVW #0x2f52 in OEM band ===")
i = 0x6d0000
while i < 0x6f0000 and i + 4 < len(img):
    w = u16(i)
    w2 = u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        i_bit = (w >> 10) & 1
        imm4 = w & 0xF
        imm3 = (w2 >> 12) & 7
        rd = (w2 >> 8) & 0xF
        imm8 = w2 & 0xFF
        imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
        if imm == 0x2F52:
            nxt = u16(i + 4)
            is_movt = (nxt & 0xFBF0) == 0xF2C0
            print(f"  @{hex(i)} MOVW r{rd},#0x2f52 followed_by_movt={is_movt}")
        i += 4
    elif (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2

# Size-related immediates near encode: look for CMP/MOVS of common header sizes 8,10,12,16,20
print("\n=== size CMPs near encode_fail lit (window) ===")
ef = str_meta.get("encode_fail")
if ef and ef["lits"]:
    lit = ef["lits"][0]
    ldrs = ldr_pc_hits_for_lit(lit)
    if ldrs:
        start = max(0, ldrs[0][0] - 0x100)
        for off, txt in disasm_window(start, 0x300):
            if any(x in txt for x in ("CMP", "MOVS", "STRB", "STRH", "STR ", "LDRB", "LDRH", "MOVW", "Unable", "IPC")):
                print(f"  {hex(off)}: {txt}")

# Search for classic SIPC fmt patterns in strings near OEM
print("\n=== nearby ASCII around catalog/encode ===")
for o in [0x6df9aa, 0x6dfcf3, 0x6dfe0d, 0x6dff00, 0x6de740]:
    print(f"@{hex(o)}: {cstr(o, 80)}")

# Look for protobuf-ish / flatbuffer / "header" size constants near utils
print("\n=== 'header' / 'length' / 'payload' strings near OEM ===")
for n in [b"header", b"payload", b"encoded size", b"message length", b"IPC header", b"app header", b"sipc", b"SIPC", b"fmt_hdr"]:
    for o in find_all(img, n, limit=8):
        if 0x6c0000 <= o <= 0x700000 or b"OEM" in img[max(0, o - 40) : o + 60]:
            print(f"  {n!r} @{hex(o)}: {cstr(max(0,o-20), 80)}")

print("\n=== verdict scaffolding ===")
print("Document only evidenced fields; leave body/header UNPROVEN if not recovered.")
print("DONE")
