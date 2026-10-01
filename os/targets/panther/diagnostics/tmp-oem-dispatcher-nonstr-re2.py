#!/usr/bin/env python3
"""Focused follow-up: catalog meta decode + INIT consumer + OEM[PB] vs SIM catalog.

Evidence only. No live I/O. No invent.
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
    return Path("C:/").joinpath(*parts)


MAIN = resolve(
    "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
img = MAIN.read_bytes()
VA0, MAIN_OFF = 0x40010000, 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN_OFF)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN_OFF


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 90) -> str | None:
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


def dec_imm(w, w2):
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    return rd, (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8


def bl_target(i: int) -> int | None:
    w, w2 = u16(i), u16(i + 2)
    if (w & 0xF800) != 0xF000 or (w2 & 0xD000) != 0xD000:
        return None
    s = (w >> 10) & 1
    imm10 = w & 0x3FF
    j1 = (w2 >> 13) & 1
    j2 = (w2 >> 11) & 1
    imm11 = w2 & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm |= ~((1 << 25) - 1)
        if imm >= (1 << 31):
            imm -= 1 << 32
    return (i + 4 + imm) & 0xFFFFFFFF


def disasm(start: int, length: int = 0x200) -> list[tuple[int, str]]:
    out: list[tuple[int, str]] = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w, w2 = u16(i), u16(i + 2)
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
                if imm <= 0x200:
                    out.append((i, f"{op} r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
                tgt = bl_target(i)
                fo = off_of(tgt) if tgt and VA0 <= tgt < VA0 + 0x8000000 else -1
                note = ""
                if fo >= 0:
                    # peek nearby strings in ±0x40 of target? no — file path via DBT later
                    note = f" file@{hex(fo)}"
                out.append((i, f"BL ->{hex(tgt) if tgt else '?'}{note}"))
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
            elif w == 0x4770:
                out.append((i, "BX lr"))
            i += 2
    return out


log(f"MAIN size={len(img)}")

# --- A) meta=0x10104 bitfield census across whole catalog bank ---
log("\n=== A) Catalog meta / +0x18 census (SIM bank 0x6de000..0x6deb00) ===")
meta_hist: dict[int, int] = {}
plus18_hist: dict[int, int] = {}
body_hist: dict[int, int] = {}
for o in range(0x6DE000, 0x6DEB00, 0x1C):
    body, mid = u16(o), u16(o + 2)
    nva = u32(o + 4)
    if not (VA0 <= nva < VA0 + 0x8000000):
        continue
    name = cstr(off_of(nva), 48)
    if not name or not name.startswith("SIM_"):
        continue
    meta = u32(o + 8)
    p18 = u32(o + 0x18)
    meta_hist[meta] = meta_hist.get(meta, 0) + 1
    plus18_hist[p18] = plus18_hist.get(p18, 0) + 1
    body_hist[body] = body_hist.get(body, 0) + 1
    if mid in (0x2F50, 0x2F51, 0x2F52, 0x2F57) or p18 != 0 or body == 2:
        log(
            f"  @{hex(o)} body={body} id={hex(mid)} meta={hex(meta)} "
            f"rsp={hex(u16(o+0xc))} +14={hex(u32(o+0x14))} +18={hex(p18)} {name}"
        )

log(f"meta_hist={ {hex(k):v for k,v in sorted(meta_hist.items())} }")
log(f"+18_hist={ {hex(k):v for k,v in sorted(plus18_hist.items())} }")
log(f"body_hist(top)={dict(sorted(body_hist.items(), key=lambda x:-x[1])[:15])}")

# Decode meta 0x10104 candidate fields
m = 0x10104
log(f"meta 0x10104 bits: lo16={hex(m & 0xFFFF)} hi16={hex(m >> 16)} byte0={m & 0xFF} byte1={(m>>8)&0xFF} byte2={(m>>16)&0xFF} byte3={(m>>24)&0xFF}")

# --- B) Deep INIT consumer @0x32e4494 (MOVW site) ---
log("\n=== B) SIM_INIT catalog consumer @0x32e4494 ===")
site = 0x32E4494
# find prolog
pro = site
for j in range(site, max(0, site - 0x400), -2):
    w = u16(j)
    if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400 or w == 0xE92D:
        pro = j
        break
log(f"prolog@{hex(pro)} va={hex(va_of(pro))}")
for off, text in disasm(pro, 0x280):
    log(f"  {hex(off)}: {text}")

# What does it load from catalog entry?
log("\nCatalog entry raw @0x6de740:")
log("  " + " ".join(f"{img[0x6de740+i]:02x}" for i in range(0x1C)))

# --- C) Compare INIT vs STOP vs INFO consumers (all flags=2) ---
log("\n=== C) MOVW sites for flags=2 siblings ===")
targets = {
    "INIT": va_of(0x6DE740),
    "STOP": va_of(0x6DE9C4),
    "INFO": va_of(0x6DE724),
}
for name, tva in targets.items():
    lo, hi = tva & 0xFFFF, (tva >> 16) & 0xFFFF
    hits = []
    o = 0x3000000
    while o < 0x3400000 - 8:
        w, w2 = u16(o), u16(o + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            if imm == lo:
                p = o + 4
                while p < o + 0x28:
                    ww, ww2 = u16(p), u16(p + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd and imm2 == hi:
                            hits.append(o)
                            break
                    if (ww & 0xF800) >= 0xE800:
                        p += 4
                    else:
                        p += 2
            o += 4
            continue
        o += 4 if (w & 0xF800) >= 0xE800 else 2
    log(f"  {name} VA={hex(tva)} sites={[hex(h) for h in hits]}")

# --- D) OEM[PB] strings — are any SIM_*? ---
log("\n=== D) [OEM][PB] string inventory (SIM overlap?) ===")
pos = 0
pb_strs = []
while True:
    i = img.find(b"[OEM][PB]", pos)
    if i < 0:
        break
    s = cstr(i, 100)
    if s:
        pb_strs.append((i, s))
    pos = i + 1
log(f"[OEM][PB] count={len(pb_strs)}")
sim_pb = [x for x in pb_strs if "SIM" in x[1].upper() or "2f50" in x[1].lower() or "INIT" in x[1]]
log(f"SIM/INIT-ish OEM[PB]: {sim_pb[:20]}")
for o, s in pb_strs[:40]:
    log(f"  @{hex(o)}: {s}")

# --- E) REQUEST enum near not-REQUEST string — scan for type constants ---
log("\n=== E) Type-enum strings near OEM IPC ---")
for nd in (
    b"REQUEST",
    b"RESPONSE",
    b"INDICATION",
    b"NOTIFICATION",
    b"IPC_MSG_TYPE",
    b"message_type",
    b"msg_type",
    b"MsgType",
):
    pos = 0
    n = 0
    while n < 12:
        i = img.find(nd, pos)
        if i < 0:
            break
        # require OEM neighborhood
        window = img[max(0, i - 80) : i + 80]
        if b"OEM" in window or b"IPC" in window or b"[OEM]" in window:
            s = cstr(i - 20 if i >= 20 else i, 80)
            log(f"  {nd!r} @{hex(i)} ctx={s!r}")
            n += 1
        pos = i + 1

# --- F) Hunt encode-size using catalog body field (LDRH [entry,#0]) ---
log("\n=== F) Catalog body_hint readers (LDRH [rn,#0] near catalog MOVW) ---")
for label, site in (("INIT", 0x32E4494), ("INFO", 0x3388338), ("VERIFY", 0x331AF14), ("FIRST", 0x331A13C)):
    log(f"\n--- {label} @{hex(site)} ±0x80 load/store catalog fields ---")
    for off, text in disasm(site - 0x40, 0x120):
        if any(x in text for x in ("LDRH", "LDRB", "STRH", "STRB", "LDR ", "STR ", "MOVW", "MOVT", "MOVS", "BL ")):
            log(f"  {hex(off)}: {text}")

# --- G) Is +0x18 a flags field used as token length? Cross-check non-SIM OEM banks ---
log("\n=== G) Non-SIM OEM catalog entries with +18!=0 (sample) ===")
# Walk larger OEM catalog region looking for name strings ending _REQ
count = 0
for o in range(0x6D8000, 0x6DF000, 0x1C):
    nva = u32(o + 4)
    if not (VA0 <= nva < VA0 + 0x8000000):
        continue
    name = cstr(off_of(nva), 48)
    if not name or not name.endswith("_REQ"):
        continue
    p18 = u32(o + 0x18)
    body = u16(o)
    mid = u16(o + 2)
    if p18 != 0:
        log(f"  @{hex(o)} body={body} id={hex(mid)} +18={hex(p18)} {name}")
        count += 1
        if count >= 40:
            break
log(f"+18!=0 REQ sample count printed={count}")

# --- H) Verdict ---
log("\n=== H) VERDICT ===")
log("flags=2 cohort: INFO(0x2f57 +18=0), INIT(0x2f50 +18=4), STOP(0x2f51 +18=4)")
log("+18=4 shared by INIT+STOP only in SIM bank — still not proven wire token length")
log("Catalog consumers remain object helpers (STRB +8/+9) — not proven app wire")
log("OEM[PB] nanopb path exists but SIM overlap check above")
log("FRAME_RECOVERED=NO — no soft SIM_INIT")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"Wrote {OUT}")
