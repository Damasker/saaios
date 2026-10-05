#!/usr/bin/env python3
"""MAIN RE: recover OEM catalog app-header + 2B body for msgid 0x2f50.

Focus:
- catalog entry @0x6de740 (cross-check 0x2f52)
- USIM SIM_INIT handler @0x43909d49
- catalog xref sites that load entry VA 0x406d7b30
- [OEM][SIT] send path / message object field stores
No live I/O. No invented wire bytes in verdict.
"""
from __future__ import annotations

import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s, flush=True)
    lines.append(s)


def resolve(*parts: str) -> Path:
    wsl = Path("/mnt/c").joinpath(*parts)
    if wsl.exists():
        return wsl
    return Path("C:\\").joinpath(*parts)


MAIN_CANDS = [
    resolve("Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    resolve("Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
img = None
main_path = None
for p in MAIN_CANDS:
    if p.exists():
        img = p.read_bytes()
        main_path = p
        break
if img is None:
    raise SystemExit(f"MAIN missing; tried: {MAIN_CANDS}")

VA0 = 0x40010000
MAIN = 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(va_or_off: int, as_va: bool = False, n: int = 80) -> str | None:
    o = off_of(va_or_off) if as_va else va_or_off
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if c == 0:
            break
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def find_all(hay: bytes, needle: bytes, limit: int = 200) -> list[int]:
    out, pos = [], 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def movw_imm(w: int, w2: int) -> tuple[int, int] | None:
    if (w & 0xFBF0) != 0xF240:
        return None
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    return rd, imm


def movt_imm(w: int, w2: int) -> tuple[int, int] | None:
    if (w & 0xFBF0) != 0xF2C0:
        return None
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
    return rd, imm


def find_movw_movt_va(target_va: int, limit: int = 40) -> list[tuple[int, int]]:
    """Find MOVW+MOVT pairs assembling target_va (any order within 16B)."""
    lo16 = target_va & 0xFFFF
    hi16 = (target_va >> 16) & 0xFFFF
    hits = []
    i = 0
    while i + 8 <= len(img) and len(hits) < limit:
        w = u16(i)
        w2 = u16(i + 2)
        mw = movw_imm(w, w2)
        if mw and mw[1] == lo16:
            rd = mw[0]
            for j in range(i + 4, min(len(img) - 3, i + 32), 2):
                w3, w4 = u16(j), u16(j + 2)
                mt = movt_imm(w3, w4)
                if mt and mt[0] == rd and mt[1] == hi16:
                    hits.append((i, j))
                    break
        i += 2
    return hits


def scan_imm_access(start: int, length: int) -> list[tuple[int, str]]:
    """Collect LDR*/STR* with small immediates + MOVW/MOVT/MOVS."""
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        # 16-bit
        if (w & 0xF800) < 0xE800:
            if (w & 0xF800) == 0x2000:  # MOVS Rd,#imm8
                out.append((i, f"MOVS r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xF800) == 0x6000:  # STR Rt,[Rn,#imm5*4]
                imm = ((w >> 6) & 0x1F) << 2
                out.append((i, f"STR r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x6800:
                imm = ((w >> 6) & 0x1F) << 2
                out.append((i, f"LDR r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x7000:
                imm = (w >> 6) & 0x1F
                out.append((i, f"STRB r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x7800:
                imm = (w >> 6) & 0x1F
                out.append((i, f"LDRB r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x8000:
                imm = ((w >> 6) & 0x1F) << 1
                out.append((i, f"STRH r{w&7},[r{(w>>3)&7},#{imm}]"))
            elif (w & 0xF800) == 0x8800:
                imm = ((w >> 6) & 0x1F) << 1
                out.append((i, f"LDRH r{w&7},[r{(w>>3)&7},#{imm}]"))
            i += 2
            continue
        # 32-bit
        mw = movw_imm(w, w2)
        if mw:
            out.append((i, f"MOVW r{mw[0]},#{hex(mw[1])}"))
            i += 4
            continue
        mt = movt_imm(w, w2)
        if mt:
            out.append((i, f"MOVT r{mt[0]},#{hex(mt[1])}"))
            i += 4
            continue
        if (w & 0xFFF0) == 0xF8C0:  # STR.W
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"STR.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFFF0) == 0xF8D0:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"LDR.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFFF0) == 0xF880:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"STRB.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFFF0) == 0xF890:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"LDRB.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFFF0) == 0xF8A0:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"STRH.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFFF0) == 0xF8B0:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x100:
                out.append((i, f"LDRH.W r{rt},[r{rn},#{imm}]"))
        elif (w & 0xFF70) == 0xF850 and (w2 & 0x0F00) == 0x0000:
            # LDR.W Rt,[Rn,Rm] etc skip
            pass
        elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
            # BL
            s = (w >> 10) & 1
            imm10 = w & 0x3FF
            j1 = (w2 >> 13) & 1
            j2 = (w2 >> 11) & 1
            imm11 = w2 & 0x7FF
            i1 = ~(j1 ^ s) & 1
            i2 = ~(j2 ^ s) & 1
            imm = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
            if imm & (1 << 24):
                imm -= 1 << 25
            target = (i + 4 + imm) & 0xFFFFFFFF
            out.append((i, f"BL {hex(va_of(target))}"))
        i += 4 if (w & 0xF800) >= 0xE800 else 2
    return out


log(f"MAIN={main_path} size={len(img)}")
log(f"VA0={hex(VA0)} MAIN_OFF={hex(MAIN)}")

# --- Catalog dump ---
CAT = 0x6DE740
log("\n=== Catalog SIM_INIT @0x6de740 / VERIFYPIN @0x6de874 ===")
for off, name in [(0x6DE740, "SIM_INIT_REQ"), (0x6DE874, "SIM_VERIFYPIN_REQ"), (0x6DE724, "SIM_INFO_REQ"), (0x6DE9C4, "SIM_STOP_REQ")]:
    body = u16(off)
    mid = u16(off + 2)
    name_va = u32(off + 4)
    meta = u32(off + 8)
    rsp = u32(off + 12)
    w4, w5, w6 = u32(off + 16), u32(off + 20), u32(off + 24)
    log(
        f"  @{hex(off)} va={hex(va_of(off))} body={body} msgid={hex(mid)} "
        f"name={cstr(name_va, True)} meta={hex(meta)} rsp={hex(rsp)} "
        f"w4={hex(w4)} w5={hex(w5)} w6={hex(w6)}"
    )
    log(f"    raw28: {' '.join(f'{b:02x}' for b in img[off:off+28])}")

# meta bit decode hypothesis dump for REQ bank
log("\n=== meta=0x10104 byte fields ===")
m = 0x10104
log(f"  LE bytes: {[hex(b) for b in struct.pack('<I', m)]}")
log("  hypotheses (unproven): b0=hdr_related? b1=msg_class(REQ=1/IND=3)? b2=1? b3=0")

# --- USIM SIM_INIT handler ---
H_VA = 0x43909D49
H_OFF = off_of(H_VA & ~1)
log(f"\n=== USIM SIM_INIT handler @{hex(H_VA)} off={hex(H_OFF)} ===")
ops = scan_imm_access(H_OFF, 0x280)
for o, s in ops[:80]:
    log(f"  {hex(o)}: {s}")

# Look for early body loads - first 0x80 of handler more carefully
log("\n=== handler early window hex ===")
log(" ".join(f"{b:02x}" for b in img[H_OFF : H_OFF + 0x80]))

# USIM msgtable entry around 0x10fb078
MT = 0x10FB070
log("\n=== USIM msgtable around SIM_INIT ===")
for o in range(0x10FB050, 0x10FB0B0, 4):
    v = u32(o)
    s = cstr(v, True) if 0x40000000 <= v <= 0x45000000 else None
    log(f"  @{hex(o)}={hex(v)} {s or ''}")

# --- Catalog entry xref ---
ENTRY_VA = 0x406D7B30  # SIM_INIT catalog
START_VA = 0x406D7AF8  # prior noted table start
log(f"\n=== MOVW+MOVT refs to catalog entry VA {hex(ENTRY_VA)} ===")
xrefs = find_movw_movt_va(ENTRY_VA, limit=20)
log(f"  hits={len(xrefs)}")
for a, b in xrefs[:12]:
    log(f"  MOVW@{hex(a)} MOVT@{hex(b)}  context:")
    for o, s in scan_imm_access(max(0, a - 0x40), 0x120)[:40]:
        log(f"    {hex(o)}: {s}")

log(f"\n=== MOVW+MOVT refs to catalog start VA {hex(START_VA)} ===")
xrefs2 = find_movw_movt_va(START_VA, limit=20)
log(f"  hits={len(xrefs2)}")
for a, b in xrefs2[:8]:
    log(f"  MOVW@{hex(a)} MOVT@{hex(b)}")
    for o, s in scan_imm_access(max(0, a - 0x20), 0x100)[:30]:
        log(f"    {hex(o)}: {s}")

# Also search bare MOVW #0x2f50 with nearby store patterns (wire encode?)
log("\n=== bare MOVW #0x2f50 sites (sample) with nearby STR/MOVS ===")
bare = []
i = 0
while i + 4 <= len(img) and len(bare) < 40:
    w, w2 = u16(i), u16(i + 2)
    mw = movw_imm(w, w2)
    if mw and mw[1] == 0x2F50:
        # skip if next is MOVT (pointer assemble)
        nxt = movt_imm(u16(i + 4), u16(i + 6)) if i + 8 <= len(img) else None
        if not (nxt and nxt[0] == mw[0]):
            bare.append(i)
    i += 2
log(f"  bare count={len(bare)} first={[hex(x) for x in bare[:16]]}")
for site in bare[:8]:
    log(f"\n  --- site {hex(site)} ---")
    for o, s in scan_imm_access(max(0, site - 0x30), 0xA0)[:35]:
        mark = " <<" if o == site else ""
        log(f"    {hex(o)}: {s}{mark}")

# --- Compare with soft SIT VerifyPin known layout vs OEM catalog body=10 ---
log("\n=== cross-check soft SIT vs OEM catalog (evidence only) ===")
log("  soft SIT VerifyPin: id=0x0201 total_len=38 on umts_ipc0 (prior RUNTIME)")
log("  OEM catalog VERIFYPIN: msgid=0x2f52 body_flags=10 rsp=0x2fa1")
log("  OEM catalog INIT: msgid=0x2f50 body_flags=2 rsp=0")
log("  SIT<->OEM dual MOVW pairs: 0 (prior) — do NOT assume soft SIT hdr carries 0x2f50")

# Search for strings about header size / fmt
log("\n=== OEM/SIT header-related strings ===")
for nd in [
    b"IpcHeader",
    b"ipc_header",
    b"message_header",
    b"MessageHeader",
    b"fmt_hdr",
    b"SIT header",
    b"header length",
    b"hdr_len",
    b"payload length",
    b"payload_len",
    b"encoded_size",
    b"GetHeader",
    b"set_message_id",
    b"message_id",
    b"msg_id",
    b"transaction",
    b"token",
]:
    hits = find_all(img, nd, limit=8)
    if hits:
        for h in hits[:3]:
            ctx = cstr(h, False, 90)
            log(f"  {nd!r} @{hex(h)}: {ctx}")

# Look near GetHeader assert for object layout comments / nearby ints
gh = img.find(b"GetHeader()->message_id == kMessageId")
if gh >= 0:
    log(f"\n=== around GetHeader assert @{hex(gh)} ===")
    # dump nearby ASCII
    for o in range(max(0, gh - 0x100), min(len(img), gh + 0x200)):
        if 32 <= img[o] < 127 and (o == 0 or img[o - 1] == 0):
            s = cstr(o)
            if s and len(s) > 8:
                log(f"  @{hex(o)}: {s}")

# Pointer-table island near encode: maybe vtable or handler table with function ptrs
log("\n=== encode ptr-table island @0x6dfc00..0x6e0240 as possible code refs via absolute ===")
# Find absolute refs TO the pointer-table slots (code that loads &table[i])
# Already hard; instead dump if any slot points into code (Thumb odd)
codeish = 0
for o in range(0x6DFC00, 0x6E0240, 4):
    v = u32(o)
    if 0x40010000 <= v <= 0x45000000 and (v & 1):
        codeish += 1
        if codeish <= 12:
            log(f"  @{hex(o)} -> {hex(v)} (thumb?) nearby_str_back={cstr(max(0,o-40))}")
log(f"  thumb-ish ptrs in island: {codeish}")

# Catalog w6=4 for INIT — appear on STOP too; INFO has 0 at +0x18
log("\n=== catalog +0x18 field across body=2 siblings ===")
for off in range(0x6DE600, 0x6DEA00, 28):
    body = u16(off)
    mid = u16(off + 2)
    name = cstr(u32(off + 4), True)
    if body == 2 and name and name.startswith("SIM_"):
        log(f"  @{hex(off)} {name} msgid={hex(mid)} +0x18={u32(off+24)} rsp={hex(u32(off+12))}")

# Verdict
log("\n=== VERDICT (no invent) ===")
log("Catalog evidenced: stride28; body_size@+0=u16; msgid@+2=u16; name_va@+4; meta@+8; rsp@+0c; +0x18 sometimes 4.")
log("App-layer wire header layout: STILL UNRECOVERED (no proven STR of msgid/len/token to buffer).")
log("2-byte body contents for 0x2f50: STILL UNRECOVERED.")
log("SENDABLE soft 0x2f50? NO")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"\nWrote {OUT}")
