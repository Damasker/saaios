#!/usr/bin/env python3
"""OEM IPC preprocess RX RE via DBT code backrefs + preprocess_cb + header loads.

Recover app-header layout / body rules for catalog msgids (incl 0x2f50).
Evidence only — no invent, no live write.
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


MAIN_CANDS = [
    resolve("Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    resolve("Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
main_path = next((p for p in MAIN_CANDS if p.exists()), None)
if main_path is None:
    raise SystemExit(f"MAIN missing: {MAIN_CANDS}")
img = main_path.read_bytes()
VA0, MAIN = 0x40010000, 0x16C10
DBT_MAGIC = 0xFECDBA98


def va_of(o: int) -> int:
    return VA0 + (o - MAIN)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 96) -> str | None:
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


def find_prolog(addr: int, back: int = 0x300) -> int:
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        # PUSH {...,lr} / PUSH.W / STMDB
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
            return j
        if w == 0xE92D:
            return j
        ww = u16(j)
        ww2 = u16(j + 2) if j + 4 <= len(img) else 0
        if ww == 0xE92D:
            return j
        # PUSH.W {regs,lr}: 0xE92D
        if (ww & 0xFFFF) == 0xE92D:
            return j
    return max(0, addr - 0x40)


def dec_imm(w: int, w2: int) -> tuple[int, int]:
    i_bit = (w >> 10) & 1
    imm4 = w & 0xF
    imm3 = (w2 >> 12) & 7
    rd = (w2 >> 8) & 0xF
    imm8 = w2 & 0xFF
    return rd, (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8


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
                if imm <= 0x100:
                    out.append((i, f"{op} r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF850 and ((w2 >> 8) & 0xF) == 0xE:
                # LDR.W Rt,[Rn,#+/-imm8] T4
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFF
                p, u, wback = (w2 >> 10) & 1, (w2 >> 9) & 1, (w2 >> 8) & 1
                sign = "+" if u else "-"
                out.append((i, f"LDR.W r{rt},[r{rn},{sign}#{imm}] p={p}w={wback}"))
            elif w == 0xF8DF or (w & 0xFF7F) == 0xF85F:
                rt = (w2 >> 12) & 0xF
                imm = w2 & 0xFFF
                u = (w >> 7) & 1
                base = (i + 4) & ~3
                lit = base + imm if u else base - imm
                val = u32(lit) if 0 <= lit < len(img) - 4 else 0
                s = cstr(off_of(val)) if VA0 <= val < VA0 + 0x8000000 else None
                out.append((i, f"LDR.W r{rt},[pc] ->lit@{hex(lit)}={hex(val)} {s!r}"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
                # BL / BLX
                s = (w >> 10) & 1
                imm10 = w & 0x3FF
                j1 = (w2 >> 13) & 1
                j2 = (w2 >> 11) & 1
                imm11 = w2 & 0x7FF
                i1 = 1 - (j1 ^ s)
                i2 = 1 - (j2 ^ s)
                imm = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
                if s:
                    imm |= ~((1 << 25) - 1) & 0xFFFFFFFF
                    imm = imm - (1 << 32) if imm >= (1 << 31) else imm
                tgt = (i + 4 + imm) & 0xFFFFFFFF
                out.append((i, f"BL ->{hex(tgt)} file@{hex(off_of(tgt)) if VA0<=tgt<VA0+0x8000000 else '?'}"))
            i += 4
        else:
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                # ARM AlignPC: (pc+4)&~3 for Thumb; try both
                lit2 = ((i + 4) & ~3) + imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                s = cstr(off_of(val)) if VA0 <= val < VA0 + 0x8000000 else None
                out.append((i, f"LDR r{rt},[pc,#{imm}] lit@{hex(lit)}={hex(val)} {s!r}"))
            elif (w & 0xF800) == 0x2000:
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


def parse_dbt_near(str_off: int, radius: int = 0x80) -> list[dict]:
    """Parse DBT records near a log string (magic 0xfecdba98)."""
    recs = []
    lo, hi = max(0, str_off - radius), min(len(img) - 16, str_off + radius)
    for o in range(lo & ~3, hi, 4):
        if u32(o) != DBT_MAGIC:
            continue
        # typical: magic, fmt_va OR code_va, ...
        # observe neighbors
        words = [u32(o + 4 * k) for k in range(6)]
        recs.append({"magic_off": o, "words": words})
    return recs


def ascii_near(o: int, radius: int = 64) -> list[str]:
    lo, hi = max(0, o - radius), min(len(img), o + radius)
    out, i = [], lo
    while i < hi:
        if 32 <= img[i] < 127:
            j = i
            while j < hi and 32 <= img[j] < 127:
                j += 1
            if j - i >= 5:
                out.append(img[i:j].decode())
            i = j
        else:
            i += 1
    return out


log(f"MAIN={main_path} size={len(img)}")

# --- A) Target strings ---
needles = {
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "preprocess_fail": b"[OEM][IPC] Failed to preprocess id %d",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "invalid": b"[OEM][IPC] Invalid message",
    "decode_fail": b"[OEM][IPC] Unable to decode message",
    "recv_partial": b"[OEM][IPC] Received ",
    "sit_recv": b"[OEM][SIT] Received packet from channel %u, size %u",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
}

log("\n=== A) String sites + DBT neighborhood ===")
str_sites: dict[str, int] = {}
for name, nd in needles.items():
    o = img.find(nd)
    if o < 0:
        log(f"{name}: MISSING")
        continue
    str_sites[name] = o
    log(f"{name}: off={hex(o)} va={hex(va_of(o))}")
    # dump 0x60 bytes before string looking for DBT
    start = max(0, (o - 0x60) & ~3)
    hexline = " ".join(f"{img[i]:02x}" for i in range(start, min(o + 4, start + 0x70)))
    log(f"  bytes@{hex(start)}: {hexline[:180]}...")
    for rec in parse_dbt_near(o, 0xA0):
        decoded = []
        for w in rec["words"]:
            if VA0 <= w < VA0 + 0x8000000:
                so = off_of(w)
                s = cstr(so, 60)
                decoded.append(f"{hex(w)}:{s!r}" if s else f"{hex(w)}:code?")
            else:
                decoded.append(hex(w))
        log(f"  DBT@{hex(rec['magic_off'])}: {decoded}")

# --- B) Broader DBT scan in OEM island 0x6df800..0x6e0400: collect code backrefs ---
log("\n=== B) OEM island DBT records with code-ish VAs ===")
code_hits: list[tuple[int, int, str]] = []  # (dbt_off, code_va, nearby_str)
for o in range(0x6DF800, 0x6E0400, 4):
    if u32(o) != DBT_MAGIC:
        continue
    # look at +/- 3 words for string VA and code VA
    window = [(o + 4 * k, u32(o + 4 * k)) for k in range(-2, 6) if 0 <= o + 4 * k < len(img) - 4]
    strs = []
    codes = []
    for off, w in window:
        if not (VA0 <= w < VA0 + 0x8000000):
            continue
        s = cstr(off_of(w), 70)
        if s and ("OEM" in s or "IPC" in s or "SIT" in s or "preprocess" in s or "encode" in s or "decode" in s or "Message" in s):
            strs.append((off, w, s))
        elif 0x41000000 <= w < 0x42000000 or 0x40600000 <= w < 0x40800000:
            # likely Thumb code in MAIN mapped range — utils near 0x41067xxx
            codes.append((off, w))
        elif MAIN <= off_of(w) < len(img) and (w & 1) == 1:
            # thumb bit set
            codes.append((off, w))
    if strs:
        tag = strs[0][2][:50]
        code_vas = [hex(c[1]) for c in codes]
        log(f"DBT@{hex(o)} strs={[s[2][:40] for s in strs[:2]]} codes={code_vas[:4]}")
        for _, cva in codes:
            code_hits.append((o, cva, tag))

# Also: scan for word == string VA near DBT, then nearby u32 that looks like code
log("\n=== B2) code VAs adjacent to msgid_nf / preprocess litptrs ===")
for name in ("msgid_nf", "preprocess_fail", "not_req", "invalid", "decode_fail", "sit_recv"):
    if name not in str_sites:
        continue
    sva = va_of(str_sites[name])
    # find litpool word == sva
    t = struct.pack("<I", sva)
    pos = 0x6DF800
    while True:
        i = img.find(t, pos, 0x6E0400)
        if i < 0:
            break
        log(f"{name} lit@{hex(i)}")
        # dump +/- 8 words
        for k in range(-4, 8):
            oo = i + 4 * k
            if oo < 0:
                continue
            w = u32(oo)
            note = ""
            if VA0 <= w < VA0 + 0x8000000:
                s = cstr(off_of(w), 50)
                if s:
                    note = f" str={s!r}"
                else:
                    # treat as potential code
                    fo = off_of(w & ~1)
                    if 0 < fo < len(img):
                        note = f" code? file@{hex(fo)}"
            log(f"  [{k:+d}] @{hex(oo)}={hex(w)}{note}")
        pos = i + 4

# --- C) preprocess_cb string sites → surrounding tables ---
log("\n=== C) preprocess_cb neighborhoods ===")
pp_sites = []
pos = 0
while True:
    i = img.find(b"preprocess_cb", pos)
    if i < 0:
        break
    pp_sites.append(i)
    pos = i + 1
for o in pp_sites:
    log(f"preprocess_cb @{hex(o)} va={hex(va_of(o))} near={ascii_near(o, 80)}")
    # dump as potential descriptor name table: look for nearby ptrs
    for k in range(-8, 12):
        oo = (o & ~3) + 4 * k
        if oo < 0 or oo + 4 > len(img):
            continue
        w = u32(oo)
        if VA0 <= w < VA0 + 0x8000000:
            s = cstr(off_of(w), 40)
            if s:
                log(f"  word@{hex(oo)} -> {hex(w)} {s!r}")

# --- D) Catalog entry decode + neighbor body_hint histogram ---
log("\n=== D) OEM catalog SIM bank (28B stride @0x6de724) ===")
# stride observed 0x1c
base = 0x6DE724
for i in range(20):
    o = base + i * 0x1C
    body = u16(o)
    mid = u16(o + 2)
    meta = u32(o + 4)
    # name ptr often via meta-adjacent? prior: raw28 showed name separately
    # From prior: raw28 = body(2) msgid(2) then 4B then meta-ish
    # Re-read: @0x6de740: 02 00 50 2f f3 05 03 41 04 01 01 00 00 00 00 00 ...
    # body=2, msgid=0x2f50, next=0x410305f3 (name VA!), then 0x10104 meta?, rsp=0
    name_va = u32(o + 4)
    meta2 = u32(o + 8)
    rsp = u16(o + 0xC)
    pad = u16(o + 0xE)
    w10 = u32(o + 0x10)
    w14 = u32(o + 0x14)
    w18 = u32(o + 0x18)
    name = cstr(off_of(name_va), 40) if VA0 <= name_va < VA0 + 0x8000000 else None
    log(
        f"  @{hex(o)} body={body} id={hex(mid)} name_va={hex(name_va)} name={name!r} "
        f"meta={hex(meta2)} rsp={hex(rsp)} +10={hex(w10)} +14={hex(w14)} +18={hex(w18)}"
    )

# --- E) Hunt catalog-base consumers: LDR of 0x6de740 VA or binary search over msgid ---
log("\n=== E) Catalog base / msgid-lookup code (MOVW 0x2f50 near LDRH compare) ===")
# Prior bare MOVW sites — re-disasm those with field loads within 0x80
movw_sites = []
i = 0
while i < len(img) - 4:
    w, w2 = u16(i), u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        rd, imm = dec_imm(w, w2)
        if imm == 0x2F50:
            # skip if next same-rd MOVT (pointer assemble)
            j = i + 4
            has_movt = False
            while j < i + 20:
                ww, ww2 = u16(j), u16(j + 2)
                if (ww & 0xFBF0) == 0xF2C0:
                    rd2, _ = dec_imm(ww, ww2)
                    if rd2 == rd:
                        has_movt = True
                        break
                if (ww & 0xF800) >= 0xE800:
                    j += 4
                else:
                    j += 2
            if not has_movt:
                movw_sites.append(i)
            i += 4
            continue
    if (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2

log(f"bare MOVW #0x2f50 count={len(movw_sites)}")

# Focus: sites that also have LDRH/LDRB with imm in {0,1,2,4,6,8,10,12} within ±0x60
Interesting = []
for site in movw_sites:
    loads = []
    for j in range(max(0, site - 0x60), min(len(img) - 4, site + 0x60), 2):
        w = u16(j)
        w2 = u16(j + 2)
        if (w & 0xF800) == 0x8800:
            imm = ((w >> 6) & 0x1F) << 1
            if imm in (0, 2, 4, 6, 8, 10, 12, 14, 16):
                loads.append((j, f"LDRH r{w&7},[r{(w>>3)&7},#{imm}]"))
        elif (w & 0xF800) == 0x7800:
            imm = (w >> 6) & 0x1F
            if imm in (0, 1, 2, 4, 5, 6, 8, 10, 12):
                loads.append((j, f"LDRB r{w&7},[r{(w>>3)&7},#{imm}]"))
        elif (w & 0xFFF0) == 0xF8B0:
            imm = w2 & 0xFFF
            if imm in (0, 2, 4, 6, 8, 10, 12, 14, 16):
                loads.append((j, f"LDRH.W r{(w2>>12)&0xF},[r{w&0xF},#{imm}]"))
        elif (w & 0xFFF0) == 0xF890:
            imm = w2 & 0xFFF
            if imm in (0, 1, 2, 4, 5, 6, 8, 10, 12):
                loads.append((j, f"LDRB.W r{(w2>>12)&0xF},[r{w&0xF},#{imm}]"))
    if loads:
        Interesting.append((site, loads))

log(f"MOVW#0x2f50 with nearby hdr-ish loads: {len(Interesting)}")
for site, loads in Interesting[:12]:
    log(f"\n--- site@{hex(site)} va={hex(va_of(site))} ---")
    for lo, txt in loads[:10]:
        log(f"  load {hex(lo)}: {txt}")
    prolog = find_prolog(site)
    for off, txt in disasm(prolog, 0x120):
        if any(k in txt for k in ("LDR", "STR", "MOVW", "MOVT", "CMP", "MOVS", "BL ")):
            log(f"  {hex(off)}: {txt}")

# --- F) Find functions that binary-search / walk catalog by comparing halfword msgid ---
# Look for MOVW catalog-base VA pieces: 0x6de740 → VA = 0x406D7B30
cat_va = va_of(0x6DE740)
log(f"\n=== F) MOVW+MOVT to catalog SIM_INIT entry VA {hex(cat_va)} ===")
lo16, hi16 = cat_va & 0xFFFF, (cat_va >> 16) & 0xFFFF
# also catalog bank start
bank_va = va_of(0x6DE724)
bank_lo, bank_hi = bank_va & 0xFFFF, (bank_va >> 16) & 0xFFFF
# and litpool island base used by RX: 0x6dff00 → VA
rx_va = va_of(0x6DFF00)
targets = {
    "cat_init": (lo16, hi16, cat_va),
    "cat_bank": (bank_lo, bank_hi, bank_va),
    "rx_island": (rx_va & 0xFFFF, (rx_va >> 16) & 0xFFFF, rx_va),
}
# Also try OEM IPC dispatcher path string VA
for name, nd in {
    "utils_c": b"oem_ipc_message_utils.c",
    "disp_c": b"oem_ipc_message_dispatcher.c",
}.items():
    o = img.find(nd)
    if o >= 0:
        v = va_of(o)
        targets[name] = (v & 0xFFFF, (v >> 16) & 0xFFFF, v)

found_tm: dict[str, list[int]] = {k: [] for k in targets}
i = 0
while i < len(img) - 8:
    w, w2 = u16(i), u16(i + 2)
    if (w & 0xFBF0) == 0xF240:
        rd, imm = dec_imm(w, w2)
        for name, (lo, hi, full) in targets.items():
            if imm != lo:
                continue
            j = i + 4
            while j < i + 24:
                ww, ww2 = u16(j), u16(j + 2)
                if (ww & 0xFBF0) == 0xF2C0:
                    rd2, imm2 = dec_imm(ww, ww2)
                    if rd2 == rd and imm2 == hi:
                        found_tm[name].append(i)
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

for name, hits in found_tm.items():
    log(f"{name}: n={len(hits)} {[hex(h) for h in hits[:10]]}")
    for h in hits[:2]:
        prolog = find_prolog(h)
        log(f"  fn~{hex(prolog)} va={hex(va_of(prolog))}")
        for off, txt in disasm(prolog, 0x180):
            if any(k in txt for k in ("LDR", "STR", "MOVW", "MOVT", "CMP", "MOVS", "BL ")):
                log(f"    {hex(off)}: {txt}")

# --- G) Shannon packed log-id style: scan for DBT emit pattern near code that compares msgid ---
# Many Shannon builds: MOVW rX, #log_id; BL dbt_trace — recover by finding
# unique constants near preprocess string's DBT record index.
log("\n=== G) Unique immediates in DBT record headers near preprocess ===")
# From prior: records look like: <hdr0> <hdr1> magic fmt_va size? code_va
# dump structured view of 0x6dfe80..0x6e0000
for o in range(0x6DFE80, 0x6E0000, 4):
    w = u32(o)
    if w == DBT_MAGIC or (VA0 <= w < VA0 + 0x8000000 and cstr(off_of(w), 20)):
        s = cstr(off_of(w), 50) if VA0 <= w < VA0 + 0x8000000 else None
        log(f"  @{hex(o)}={hex(w)} {s!r}" if s else f"  @{hex(o)}={hex(w)}")

# --- H) C++ GetHeader path — object layout near assert ---
log("\n=== H) GetHeader / message_id typed IPC (may be protobuf SitOem, not catalog) ===")
gh = img.find(b"GetHeader()->message_id == kMessageId")
if gh >= 0:
    log(f"assert@{hex(gh)} near={ascii_near(gh, 100)}")
    # find litrefs
    gva = va_of(gh)
    t = struct.pack("<I", gva)
    refs = []
    pos = 0
    while len(refs) < 8:
        i = img.find(t, pos)
        if i < 0:
            break
        refs.append(i)
        pos = i + 4
    log(f"litrefs={[hex(r) for r in refs]}")

# --- I) Compare body_hint vs encode: look for CMP #body near catalog walk ---
# For SIM_INIT body=2: find functions that CMP Rn,#2 then use catalog ptr
log("\n=== I) Kernel wrap reminder (from live DT prior) ===")
log("oem_ipc attrs=0x2000 ch=0x81 — no ATTR_NO_LINK_HEADER => EXYNOS 12B wrap by kernel")
log("userspace write = app payload only")

# --- J) Attempt: find BL targets from DBT code fields that look like Thumb+1 ---
log("\n=== J) Disasm candidate code VAs from B2 / DBT ===")
# Re-parse msgid_nf neighborhood more carefully for Thumb addresses
candidate_fns = set()
for name in ("msgid_nf", "preprocess_fail", "not_req"):
    if name not in str_sites:
        continue
    sva = va_of(str_sites[name])
    t = struct.pack("<I", sva)
    i = img.find(t, 0x6DF800, 0x6E0400)
    if i < 0:
        continue
    for k in range(-6, 10):
        oo = i + 4 * k
        w = u32(oo)
        if VA0 <= w < VA0 + 0x8000000:
            fo = off_of(w & ~1)
            # reject if it's another string
            if cstr(fo, 8):
                continue
            # accept if looks like code: PUSH nearby or valid Thumb
            if 0x100000 < fo < 0x5000000:
                candidate_fns.add(fo)

# Also search entire image for u32 == (code candidate | 1) near OEM band — skip heavy
# Instead: look for ADR-style offsets from code in 0x32xxxx..0x34xxxx (common for oem handlers from prior)
# Prior encode island pointed at oem_ipc_message_utils — search file path lit and DBT code near utils
utils_o = img.find(b"oem_ipc_message_utils.c")
if utils_o >= 0:
    log(f"utils.c @{hex(utils_o)} va={hex(va_of(utils_o))}")
    # DBT often embeds file VA; find magic near references to this VA
    uva = va_of(utils_o)
    t = struct.pack("<I", uva)
    pos = 0
    n = 0
    while n < 30:
        i = img.find(t, pos)
        if i < 0:
            break
        # look back for magic within 32B
        for back in range(0, 48, 4):
            if u32(i - back) == DBT_MAGIC:
                # words after magic
                code_cand = u32(i - back + 4)
                fmt = u32(i - back + 8) if False else None
                words = [u32(i - back + 4 * k) for k in range(0, 5)]
                log(f"  utils DBT@{hex(i-back)} words={[hex(x) for x in words]}")
                for w in words:
                    if VA0 <= w < VA0 + 0x8000000 and not cstr(off_of(w & ~1), 6):
                        candidate_fns.add(off_of(w & ~1))
                break
        pos = i + 4
        n += 1

log(f"\ncandidate_fns count={len(candidate_fns)}")
for fo in sorted(candidate_fns)[:20]:
    # verify thumb-ish
    prolog = find_prolog(fo)
    log(f"\n--- cand file@{hex(fo)} va={hex(va_of(fo))} prolog@{hex(prolog)} ---")
    field_loads = []
    for off, txt in disasm(prolog, 0x200):
        if "LDR" in txt or "STR" in txt or "CMP" in txt or "MOVW" in txt or "MOVS" in txt:
            log(f"  {hex(off)}: {txt}")
            if "LDRH" in txt or "LDRB" in txt:
                field_loads.append(txt)
    if field_loads:
        log(f"  FIELD_LOADS: {field_loads[:12]}")

# --- K) Verdict ---
log("\n=== K) VERDICT INPUTS ===")
log("Need evidenced: app hdr field order/size, 2B body for flags=2, token/seq, oem_ipcN")
log("Do NOT invent SIT-12B reuse; soft VerifyPin ≠ OEM VERIFYPIN body")
log("DONE")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"\nWrote {OUT}", flush=True)
