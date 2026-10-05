#!/usr/bin/env python3
"""Follow encode-island thumb ptrs + MessageInfo size path for OEM header layout."""
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
    return wsl if wsl.exists() else Path("C:\\").joinpath(*parts)


MAIN = resolve(
    "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = MAIN.read_bytes()
VA0 = 0x40010000
BASE = 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - BASE)


def off_of(va: int) -> int:
    return (va - VA0) + BASE


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 100) -> str | None:
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


def cstr_va(va: int) -> str | None:
    return cstr(off_of(va))


def movw_imm(w, w2):
    if (w & 0xFBF0) != 0xF240:
        return None
    i = (w >> 10) & 1
    imm = (i << 11) | ((w & 0xF) << 12) | (((w2 >> 12) & 7) << 8) | (w2 & 0xFF)
    return (w2 >> 8) & 0xF, imm


def movt_imm(w, w2):
    if (w & 0xFBF0) != 0xF2C0:
        return None
    i = (w >> 10) & 1
    imm = (i << 11) | ((w & 0xF) << 12) | (((w2 >> 12) & 7) << 8) | (w2 & 0xFF)
    return (w2 >> 8) & 0xF, imm


def disasm(start: int, length: int = 0x180):
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w, w2 = u16(i), u16(i + 2)
        if (w & 0xF800) < 0xE800:
            if (w & 0xF800) == 0x2000:
                out.append((i, f"MOVS r{(w>>8)&7},#{w&0xFF}"))
            elif (w & 0xFF00) == 0xB500:
                out.append((i, "PUSH {..,lr}"))
            elif (w & 0xFF00) == 0xB400:
                out.append((i, "PUSH"))
            elif (w & 0xF800) == 0x6000:
                out.append((i, f"STR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xF800) == 0x6800:
                out.append((i, f"LDR r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<2}]"))
            elif (w & 0xF800) == 0x7000:
                out.append((i, f"STRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x7800:
                out.append((i, f"LDRB r{w&7},[r{(w>>3)&7},#{(w>>6)&0x1F}]"))
            elif (w & 0xF800) == 0x8000:
                out.append((i, f"STRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xF800) == 0x8800:
                out.append((i, f"LDRH r{w&7},[r{(w>>3)&7},#{((w>>6)&0x1F)<<1}]"))
            elif (w & 0xFF00) == 0xBD00:
                out.append((i, "POP {..,pc}"))
            i += 2
            continue
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
        if (w & 0xFFF0) == 0xF8C0:
            out.append((i, f"STR.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xFFF0) == 0xF8D0:
            out.append((i, f"LDR.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xFFF0) == 0xF880:
            out.append((i, f"STRB.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xFFF0) == 0xF890:
            out.append((i, f"LDRB.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xFFF0) == 0xF8A0:
            out.append((i, f"STRH.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xFFF0) == 0xF8B0:
            out.append((i, f"LDRH.W r{(w2>>12)&0xF},[r{w&0xF},#{w2&0xFFF}]"))
        elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
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
            tgt = i + 4 + imm
            out.append((i, f"BL {hex(va_of(tgt))}"))
        elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0x8000:
            out.append((i, f"B.W/other {hex(w)} {hex(w2)}"))
        i += 4 if (w & 0xF800) >= 0xE800 else 2
    return out


log(f"MAIN={MAIN} size={len(img)}")

# Dump encode island with string + code ptrs
log("\n=== encode island 0x6dfc00..0x6e0280 structured ===")
o = 0x6DFC00
while o < 0x6E0280:
    v = u32(o)
    s = None
    if 0x40010000 <= v < 0x45000000:
        s = cstr_va(v)
        if not s and (v & 1):
            s = f"CODE?{hex(v)}"
    if s or (0x40010000 <= v < 0x45000000):
        log(f"  @{hex(o)} -> {hex(v)} {s or ''}")
    o += 4

# Disasm interesting code ptrs from island
code_ptrs = []
for o in range(0x6DFC00, 0x6E0280, 4):
    v = u32(o)
    if 0x42000000 <= v <= 0x44000000 and (v & 1):
        code_ptrs.append((o, v))

log(f"\n=== disasm code ptrs from island (n={len(code_ptrs)}) ===")
for slot, va in code_ptrs[:16]:
    off = off_of(va & ~1)
    log(f"\n--- slot@{hex(slot)} -> {hex(va)} off={hex(off)} ---")
    # nearby ascii before function
    for back in range(0, 0x40, 4):
        pass
    for a, s in disasm(off, 0x100)[:40]:
        log(f"  {hex(a)}: {s}")

# Look at utils.c / dispatcher.c nearby - find ASCII path then xref via litpool absolute
utils_off = img.find(b"oem_ipc_message_utils.c")
disp_off = img.find(b"oem_ipc_message_dispatcher.c")
log(f"\nutils.c @{hex(utils_off)} va={hex(va_of(utils_off))}")
log(f"disp.c @{hex(disp_off)} va={hex(va_of(disp_off))}")

# Search absolute litpools containing utils/disp VAs
for name, off in [("utils", utils_off), ("disp", disp_off)]:
    va = va_of(off)
    pat = struct.pack("<I", va)
    hits = []
    pos = 0
    while len(hits) < 20:
        i = img.find(pat, pos)
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    log(f"  litrefs to {name} VA {hex(va)}: {[hex(h) for h in hits[:12]]}")

# MessageInfo size assert neighbor - search for SIM_INIT typed size tables
# Pattern: kMessageInfo -> size field; look for table of {msgid, size} near OEM SIM names
log("\n=== hunt MessageInfo-like {msgid,size} near catalog / typed IPC ===")
# Already have catalog. Check assert string refs via litpool
assert1 = img.find(b"GetDataSpan().size() == kMessageInfo")
assert2 = img.find(b"GetHeader()->message_id == kMessageId")
log(f"size assert @{hex(assert1) if assert1>=0 else None} va={hex(va_of(assert1)) if assert1>=0 else None}")
log(f"msgid assert @{hex(assert2) if assert2>=0 else None}")

# For size assert VA, find litpool
if assert1 >= 0:
    va = va_of(assert1)
    pat = struct.pack("<I", va)
    pos = 0
    hits = []
    while len(hits) < 10:
        i = img.find(pat, pos)
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    log(f"  litrefs size-assert: {[hex(h) for h in hits]}")
    for h in hits[:4]:
        # dump surrounding as ptr table
        log(f"  around lit @{hex(h)}:")
        for o in range(h - 0x20, h + 0x40, 4):
            v = u32(o)
            s = cstr_va(v) if 0x40000000 <= v <= 0x45000000 else None
            log(f"    @{hex(o)}={hex(v)} {s or ''}")

# USIM size field interpretation
log("\n=== USIM msgtable size field vs catalog body ===")
# Walk a few entries: pattern from prior dump
# Looking at structure more carefully around 0x10fb050
entries = [
    (0x10FB05C, "INFO path"),
    (0x10FB078, "INIT path"),
]
# Better: scan consecutive msgid-looking 0x2fxx with following name VA
o = 0x10FB000
found = []
while o < 0x10FB200 and len(found) < 20:
    mid = u32(o)
    if 0x2F00 <= mid <= 0x2FFF:
        name_va = u32(o + 4)
        name = cstr_va(name_va) if 0x41000000 <= name_va <= 0x41100000 else None
        if name and "SIM_" in name:
            # guess layout: msgid, name, [rsp_or_handler], ...
            w2, w3, w4, w5 = u32(o + 8), u32(o + 12), u32(o + 16), u32(o + 20)
            found.append((o, mid, name, w2, w3, w4, w5))
    o += 4
for o, mid, name, w2, w3, w4, w5 in found:
    log(f"  @{hex(o)} mid={hex(mid)} {name} +8={hex(w2)} +c={hex(w3)} +10={hex(w4)} +14={hex(w5)}")

# Compare INIT internal size 0x10000 vs catalog body 2
log("\nNOTE: USIM table 0x10000 after INIT handler vs catalog body=2 — different layers;")
log("catalog body is OEM-encoded size hint; USIM 0x10000 may be internal msg size flags.")

# Disasm helper BL 0x42ca1d2e from catalog xref (message factory?)
FACTORY = off_of(0x42CA1D2E & ~1)
log(f"\n=== catalog helper BL target 0x42ca1d2e off={hex(FACTORY)} ===")
for a, s in disasm(FACTORY, 0x120)[:50]:
    log(f"  {hex(a)}: {s}")

# Real cbd confirmation summary from carved file
cbd = resolve(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-cbd/carved-cbd-a45e000.elf"
)
if cbd.exists():
    data = cbd.read_bytes()
    log(f"\n=== carved real cbd @{cbd.name} size={len(data)} ===")
    for nd in [b"cbd:", b"/dev/umts_boot0", b"/dev/umts_ramdump0", b"/dev/oem_ipc", b"SIM_", b"rild", b"write(", b"ioctl"]:
        log(f"  {nd!r}: {data.count(nd) if len(nd)<6 else (1 if nd in data else 0)}")
    # list interesting strings
    for nd in [b"S5300", b"boot stage", b"ramdump", b"POWER", b"cp_crash", b"oem"]:
        i = data.find(nd)
        if i >= 0:
            ctx = "".join(chr(c) if 32 <= c < 127 else "." for c in data[i : i + 70])
            log(f"  hit {nd!r} @{hex(i)}: {ctx}")

log("\n=== VERDICT ===")
log("App header wire layout: still unrecovered unless encode-island CODE disasm proves stores.")
log("cbd = boot/ramdump only; no 0x2f50 encode.")
log("SENDABLE? NO without proven header+2B body")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"Wrote {OUT}", flush=True)
