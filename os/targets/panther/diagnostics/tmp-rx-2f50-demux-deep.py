#!/usr/bin/env python3
"""Deep RX demux: OEM/SIT receive path + catalog lookup + VERIFYPIN table.

Follow litpool at Received-packet string; hunt LDRH msgid@+2 / len@+4
patterns near OEM SIT island; dump USIM VERIFYPIN table entry.
"""
from __future__ import annotations

import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s, flush=True)
    lines.append(s)


img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA0, MAIN = 0x40010000, 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN


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


def find_all(needle: bytes, limit: int = 50) -> list[int]:
    hits, pos = [], 0
    while len(hits) < limit:
        i = img.find(needle, pos)
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    return hits


def dump_thumb(o: int, n: int = 0x120, label: str = "") -> None:
    log(f"--- {label} @{hex(o)} va={hex(va_of(o))} ---")
    end = min(len(img) - 4, o + n)
    i = o & ~1
    while i < end:
        w, w2 = u16(i), u16(i + 2)
        note = ""
        if (w & 0xFBF0) == 0xF240:
            ib = (w >> 10) & 1
            imm = (ib << 11) | ((w & 0xF) << 12) | (((w2 >> 12) & 7) << 8) | (w2 & 0xFF)
            rd = (w2 >> 8) & 0xF
            note = f";MOVW r{rd},#{hex(imm)}"
            log(f" {hex(i)}: {w:04x} {w2:04x} {note}")
            i += 4
            continue
        if (w & 0xFBF0) == 0xF2C0:
            ib = (w >> 10) & 1
            imm = (ib << 11) | ((w & 0xF) << 12) | (((w2 >> 12) & 7) << 8) | (w2 & 0xFF)
            rd = (w2 >> 8) & 0xF
            note = f";MOVT r{rd},#{hex(imm)}"
            log(f" {hex(i)}: {w:04x} {w2:04x} {note}")
            i += 4
            continue
        # LDR.W Rt,[pc,#imm] T2: F8DF
        if w == 0xF8DF or (w & 0xFF7F) == 0xF85F:
            # approximate
            pass
        if (w & 0xF800) == 0x4800:  # LDR Rt,[pc,#imm] T1
            rt = (w >> 8) & 7
            imm = (w & 0xFF) << 2
            pc = (i + 4) & ~3
            tgt = pc + imm
            val = u32(tgt) if tgt + 4 <= len(img) else 0
            s = cstr(off_of(val)) if VA0 <= val < VA0 + 0x8000000 else None
            note = f";LDR r{rt},[pc,#{imm}] -> {hex(val)} {s!r}"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        if (w & 0xF800) == 0x8800:
            imm5 = (w >> 6) & 0x1F
            rn = (w >> 3) & 7
            rt = w & 7
            note = f";LDRH r{rt},[r{rn},#{imm5*2}]"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        if (w & 0xF800) == 0x7800:
            imm5 = (w >> 6) & 0x1F
            rn = (w >> 3) & 7
            rt = w & 7
            note = f";LDRB r{rt},[r{rn},#{imm5}]"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        if (w & 0xF800) == 0x2800:
            note = f";CMP r{(w>>8)&7},#{w&0xFF}"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        # BL
        if (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
            s = (w >> 10) & 1
            imm10 = w & 0x3FF
            j1 = (w2 >> 13) & 1
            j2 = (w2 >> 11) & 1
            imm11 = w2 & 0x7FF
            i1 = 1 - (j1 ^ s)
            i2 = 1 - (j2 ^ s)
            imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
            if s:
                imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
                imm32 = imm32 - (1 << 32) if imm32 >= (1 << 31) else imm32
            tgt = (i + 4 + imm32) & 0xFFFFFFFF
            note = f";BL ->{hex(tgt)} file@{hex(off_of(tgt)) if VA0<=tgt<VA0+0x8000000 else '?'}"
            log(f" {hex(i)}: {w:04x} {w2:04x} {note}")
            i += 4
            continue
        if (w & 0xF800) >= 0xE800:
            log(f" {hex(i)}: {w:04x} {w2:04x}")
            i += 4
        else:
            log(f" {hex(i)}: {w:04x}")
            i += 2


log("=== 1) RX string litpool island @0x6dffe4 ===")
# pointer at 0x6dffe4 -> string VA
for o in range(0x6DFC00, 0x6E0300, 4):
    w = u32(o)
    if VA0 <= w < VA0 + 0x2000000:
        s = cstr(off_of(w), 80)
        if s and ("OEM" in s or "Received" in s or "Sending" in s or "channel" in s or "encode" in s or "REQUEST" in s):
            log(f"  @{hex(o)} -> {hex(w)} {s!r}")

# Find Thumb code that ADRs/LDRs the lit at 0x6dffe4
# Scan for LDR pc-rel that lands on 0x6dffe4
log("\n=== 2) Code refs into RX litpool (0x6dff00..0x6e0200) ===")
code_hits = []
i = 0
# Restrict scan to nearby code banks — search whole image for pc-rel LDR to pool
# Heuristic: look for BL targets / functions within 0x1000 of pool — often same .text island
# Instead: scan ±0x8000 for PUSH {..} prologues and dump those with LDRH #2/#4
band_lo, band_hi = 0x6D0000, 0x6F0000
# Also search for MOVW/MOVT constructing 0x406d936c (recv string VA)
recv_va = va_of(0x6DFF7C)
log(f"recv_str_va={hex(recv_va)}")
# Find MOVW of low 16 + MOVT high of recv_va
lo16 = recv_va & 0xFFFF
hi16 = (recv_va >> 16) & 0xFFFF
movw_sites = []
i = 0
while i < len(img) - 8:
    w, w2 = u16(i), u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        ib = (w >> 10) & 1
        imm = (ib << 11) | ((w & 0xF) << 12) | (((w2 >> 12) & 7) << 8) | (w2 & 0xFF)
        rd = (w2 >> 8) & 0xF
        if imm == lo16:
            # look ahead for MOVT same rd with hi16
            for j in range(i + 4, min(i + 24, len(img) - 4), 2):
                ww, ww2 = u16(j), u16(j + 2)
                if (ww & 0xFBF0) == 0xF2C0 and ((ww2 >> 8) & 0xF) == rd:
                    ib2 = (ww >> 10) & 1
                    imm2 = (ib2 << 11) | ((ww & 0xF) << 12) | (((ww2 >> 12) & 7) << 8) | (ww2 & 0xFF)
                    if imm2 == hi16:
                        movw_sites.append(i)
                    break
                if (ww & 0xF800) < 0xE800:
                    continue
        i += 4
    elif (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2
log(f"MOVW+MOVT to recv_str: n={len(movw_sites)} {[hex(x) for x in movw_sites[:20]]}")
for o in movw_sites[:6]:
    dump_thumb(max(0, o - 0x40), 0x100, f"recv_xref@{hex(o)}")

# Catalog walk helper: find code loading catalog base 0x406d7af8 / 0x406d7b30
log("\n=== 3) Catalog base xrefs (for RX lookup) ===")
for target in (0x406D7AF8, 0x406D7B30, 0x406D7C64):
    refs = find_all(struct.pack("<I", target), 30)
    log(f"  lit {hex(target)}: {[hex(r) for r in refs[:15]]}")
    for r in refs[:4]:
        # dump nearby if looks like code (preceded by thumb-ish)
        dump_thumb(max(0, r - 0x60), 0xA0, f"lit@{hex(r)}->{hex(target)}")

# USIM VERIFYPIN table
log("\n=== 4) USIM VERIFYPIN table @ name litref ===")
vp = img.find(b"USIM <== SIM_VERIFYPIN_REQ")
name_va = va_of(vp)
log(f"str@{hex(vp)} va={hex(name_va)}")
for r in find_all(struct.pack("<I", name_va), 10):
    # dump 32B before/after
    blob = img[r - 16 : r + 32]
    log(f"  hit@{hex(r)} mid_before={hex(u32(r-4))} handler={hex(u32(r+4))} size={hex(u32(r+8))}")
    log(f"    words={[hex(u32(r-16+i)) for i in range(0,48,4)]}")

# SIM_INIT handler prologue: does it read inbound body?
log("\n=== 5) SIM_INIT handler @0x43909d49 — early loads from args ===")
h_off = off_of(0x43909D49 & ~1)
dump_thumb(h_off, 0x100, "SIM_INIT_handler")

# Hunt SIT-like header parse: LDRB #0, LDRH #2, LDRH #4 clustered
log("\n=== 6) Cluster: LDRB+0 / LDRH+2 / LDRH+4 within 32B (SIT-like parse) near OEM island ===")
# scan band around oem sit strings 0x6d0000-0x6f0000 and also 0xc40000 (typed ipc)
for band in ((0x6D8000, 0x6E8000), (0xC40000, 0xC50000), (0x32E0000, 0x3320000)):
    hits = []
    o = band[0] & ~1
    while o < band[1] - 0x40 and len(hits) < 25:
        # look for LDRB [rn,#0] then within 32B LDRH [rm,#2] and LDRH [rk,#4]
        w = u16(o)
        if (w & 0xF800) == 0x7800 and ((w >> 6) & 0x1F) == 0:
            rn0 = (w >> 3) & 7
            window = img[o : o + 0x30]
            has2 = has4 = False
            for j in range(0, len(window) - 1, 2):
                ww = struct.unpack_from("<H", window, j)[0]
                if (ww & 0xF800) == 0x8800:
                    offb = ((ww >> 6) & 0x1F) * 2
                    if offb == 2:
                        has2 = True
                    if offb == 4:
                        has4 = True
            if has2 and has4:
                hits.append(o)
        if (w & 0xF800) >= 0xE800:
            o += 4
        else:
            o += 2
    log(f"  band {hex(band[0])}..{hex(band[1])}: n={len(hits)} first={[hex(h) for h in hits[:10]]}")
    for h in hits[:3]:
        dump_thumb(h, 0x60, f"sit_parse@{hex(h)}")

# meta byte meaning: compare body sizes vs soft SIT known msgs if any dual
log("\n=== 7) Kernel path (evidence from live DT + prior modem.md) ===")
log("  oem_ipc DT: attrs=0x2000 ch=0x81 fmt=0 ch_count=8 io_type=1 link_type=1")
log("  umts_ipc DT: attrs=0x2000 ch=0xf5 fmt=0 ch_count=2")
log("  ATTR_NO_LINK_HEADER=0x100 NOT set => link_header=true (same as boot0 path)")
log("  Kernel: userspace write = APP payload; EXYNOS 12B prepended (sync=0xABCD,...)")
log("  Channel byte in EXYNOS hdr = iod ch (0x81 for oem_ipc family)")

log("\n=== 8) Exact missing after RX pass ===")
log("DONE")
OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"wrote {OUT}")
