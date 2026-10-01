#!/usr/bin/env python3
"""Non-string RE: OEM catalog walk / nanopb / dispatcher REQUEST frame parse.

Recover how REQUEST frames are parsed — header fields, body for flags=2
msgids (esp. 0x2f50). No string-xref-only reliance. Evidence only. No live I/O.
"""
from __future__ import annotations

import struct
from collections import Counter
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
    resolve(
        "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
    ),
    resolve("Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
main_path = next((p for p in MAIN_CANDS if p.exists()), None)
if main_path is None:
    raise SystemExit(f"MAIN missing: {MAIN_CANDS}")

img = main_path.read_bytes()
VA0, MAIN_OFF = 0x40010000, 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN_OFF)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN_OFF


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 80) -> str | None:
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


def find_movw_movt_va(target_va: int, band: tuple[int, int], limit: int = 40) -> list[int]:
    lo, hi = target_va & 0xFFFF, (target_va >> 16) & 0xFFFF
    refs: list[int] = []
    o = band[0]
    while o < band[1] - 8 and len(refs) < limit:
        w, w2 = u16(o), u16(o + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            if imm == lo:
                p = o + 4
                end = min(o + 0x30, band[1] - 4)
                while p < end:
                    ww, ww2 = u16(p), u16(p + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd and imm2 == hi:
                            refs.append(o)
                            break
                    if (ww & 0xF800) >= 0xE800:
                        p += 4
                    else:
                        p += 2
            o += 4
            continue
        if (w & 0xF800) >= 0xE800:
            o += 4
        else:
            o += 2
    return refs


def litpool_hits(target_va: int, band: tuple[int, int], limit: int = 60) -> list[int]:
    hits: list[int] = []
    t = struct.pack("<I", target_va & 0xFFFFFFFF)
    pos = band[0]
    while len(hits) < limit:
        i = img.find(t, pos, band[1])
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    return hits


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


def find_prolog(addr: int, back: int = 0x400) -> int:
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
            return j
        if w == 0xE92D:
            return j
    return max(0, addr - 0x40)


def disasm_window(start: int, length: int = 0x120) -> list[tuple[int, str]]:
    """Compact Thumb disasm focused on loads/stores/immediates/BL."""
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
                if imm <= 0x180:
                    out.append((i, f"{op} r{rt},[r{rn},#{imm}]"))
            elif (w & 0xFFF0) == 0xF850 and ((w2 >> 8) & 0xF) == 0xE:
                rt = (w2 >> 12) & 0xF
                rn = w & 0xF
                imm = w2 & 0xFF
                u = (w2 >> 9) & 1
                sign = "+" if u else "-"
                out.append((i, f"LDR.W r{rt},[r{rn},{sign}#{imm}]"))
            elif w == 0xF8DF or (w & 0xFF7F) == 0xF85F:
                rt = (w2 >> 12) & 0xF
                imm = w2 & 0xFFF
                u = (w >> 7) & 1
                base = (i + 4) & ~3
                lit = base + imm if u else base - imm
                val = u32(lit) if 0 <= lit < len(img) - 4 else 0
                note = ""
                if VA0 <= val < VA0 + 0x8000000:
                    s = cstr(off_of(val), 40)
                    note = f" {s!r}" if s else f" file@{hex(off_of(val))}"
                out.append((i, f"LDR.W r{rt},[pc] lit@{hex(lit)}={hex(val)}{note}"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
                tgt = bl_target(i)
                if tgt is not None:
                    fo = off_of(tgt) if VA0 <= tgt < VA0 + 0x8000000 else -1
                    out.append((i, f"BL ->{hex(tgt)} file@{hex(fo) if fo>=0 else '?'}"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0x8000:
                # B.W
                out.append((i, "B.W"))
            i += 4
        else:
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                note = ""
                if VA0 <= val < VA0 + 0x8000000:
                    s = cstr(off_of(val), 40)
                    note = f" {s!r}" if s else f" file@{hex(off_of(val))}"
                out.append((i, f"LDR r{rt},[pc,#{imm}] lit@{hex(lit)}={hex(val)}{note}"))
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
            elif (w & 0xFF00) == 0xBD00 or (w & 0xFE00) == 0xBC00:
                out.append((i, f"POP {hex(w)}"))
            elif w == 0x4770:
                out.append((i, "BX lr"))
            i += 2
    return out


def collect_hdr_loads(start: int, length: int = 0x200) -> list[tuple[int, str, int]]:
    """Collect LDRB/LDRH/LDR with small imm — candidate wire-header field reads."""
    loads: list[tuple[int, str, int]] = []
    for off, text in disasm_window(start, length):
        if text.startswith(("LDRB", "LDRH", "LDR ", "LDR.W", "LDRB.W", "LDRH.W")):
            # extract imm if present
            if "#" in text and ",#" in text.replace(" ", ""):
                try:
                    imm_s = text.split("#")[-1].split("]")[0].split(" ")[0]
                    imm_s = imm_s.lstrip("+-")
                    imm = int(imm_s, 0) if imm_s else -1
                except ValueError:
                    imm = -1
            else:
                imm = -1
            if 0 <= imm <= 0x40:
                loads.append((off, text, imm))
        elif text.startswith(("STRB", "STRH", "STR ", "STR.W", "STRB.W", "STRH.W")):
            if "#" in text:
                try:
                    imm_s = text.split("#")[-1].split("]")[0].split(" ")[0]
                    imm_s = imm_s.lstrip("+-")
                    imm = int(imm_s, 0) if imm_s else -1
                except ValueError:
                    imm = -1
                if 0 <= imm <= 0x40:
                    loads.append((off, text, imm))
    return loads


log(f"MAIN={main_path} size={len(img)}")

# ---------------------------------------------------------------------------
# A) Catalog walk — full SIM OEM bank + flags=2 cohort
# ---------------------------------------------------------------------------
log("\n=== A) Catalog walk SIM bank (28B stride) + flags=2 cohort ===")
# Discover stride / bank start by scanning for SIM_* name VAs
candidates: list[int] = []
for o in range(0x6DE000, 0x6DF000, 4):
    w = u32(o)
    if not (VA0 <= w < VA0 + 0x8000000):
        continue
    s = cstr(off_of(w), 40)
    if s and s.startswith("SIM_") and s.endswith("_REQ"):
        candidates.append(o - 4)  # body/msgid likely just before name_va

# Confirm stride from consecutive hits
if len(candidates) >= 2:
    deltas = [candidates[i + 1] - candidates[i] for i in range(len(candidates) - 1)]
    stride = Counter(deltas).most_common(1)[0][0]
else:
    stride = 0x1C
log(f"SIM_*_REQ name-adjacent candidates={len(candidates)} stride_mode={hex(stride)}")

# Parse entries: prior layout = body:u16, msgid:u16, name_va:u32, meta:u32, rsp:u16, ...
entries: list[dict] = []
seen = set()
for base in candidates:
    o = base
    if o in seen:
        continue
    body = u16(o)
    mid = u16(o + 2)
    name_va = u32(o + 4)
    meta = u32(o + 8)
    rsp = u16(o + 0xC)
    w10 = u32(o + 0x10)
    w14 = u32(o + 0x14)
    w18 = u32(o + 0x18)
    name = cstr(off_of(name_va), 48) if VA0 <= name_va < VA0 + 0x8000000 else None
    if not name or not name.startswith("SIM_"):
        continue
    seen.add(o)
    rec = {
        "off": o,
        "body": body,
        "msgid": mid,
        "name": name,
        "meta": meta,
        "rsp": rsp,
        "w10": w10,
        "w14": w14,
        "w18": w18,
    }
    entries.append(rec)

for e in sorted(entries, key=lambda x: x["msgid"]):
    mark = " <<<" if e["msgid"] in (0x2F50, 0x2F52, 0x2F57, 0x2F58) else ""
    log(
        f"  @{hex(e['off'])} body={e['body']} id={hex(e['msgid'])} meta={hex(e['meta'])} "
        f"rsp={hex(e['rsp'])} +10={hex(e['w10'])} +14={hex(e['w14'])} +18={hex(e['w18'])} "
        f"{e['name']}{mark}"
    )

flags2 = [e for e in entries if e["body"] == 2]
log(f"\nflags/body_hint==2 count={len(flags2)}: {[e['name'] for e in flags2]}")
for e in flags2:
    log(
        f"  flags2 @{hex(e['off'])} id={hex(e['msgid'])} +18={hex(e['w18'])} "
        f"meta={hex(e['meta'])} {e['name']}"
    )

# ---------------------------------------------------------------------------
# B) Catalog pointer consumers (non-string): litpool + MOVW/MOVT to bank VAs
# ---------------------------------------------------------------------------
log("\n=== B) Catalog VA consumers (litpool + MOVW/MOVT) ===")
# Bank base candidates: first SIM entry, SIM_INIT, and round page
bank_vas = {
    "sim_info_req": va_of(0x6DE724),
    "sim_init_req": va_of(0x6DE740),
    "sim_verifypin": va_of(0x6DE874),
    "bank_page": va_of(0x6DE000),
    "bank_6de700": va_of(0x6DE700),
}
# Also try array-start: walk backward from SIM_INFO for contiguous records
scan_start = 0x6DE000
while scan_start < 0x6DE724:
    body = u16(scan_start)
    mid = u16(scan_start + 2)
    nva = u32(scan_start + 4)
    if (
        body < 0x200
        and 0x2F00 <= mid <= 0x2FFF
        and VA0 <= nva < VA0 + 0x8000000
        and (cstr(off_of(nva), 20) or "").startswith("SIM_")
    ):
        bank_vas["first_sim"] = va_of(scan_start)
        break
    scan_start += 4

CODE_BAND = (0x200000, 0x3A00000)
for name, tva in bank_vas.items():
    movs = find_movw_movt_va(tva, CODE_BAND, limit=30)
    lits = litpool_hits(tva, CODE_BAND, limit=40)
    log(f"  {name} VA={hex(tva)} movw={len(movs)} {[hex(x) for x in movs[:10]]}")
    log(f"    lit={len(lits)} {[hex(x) for x in lits[:12]]}")

# ---------------------------------------------------------------------------
# C) Binary-search / table-walk patterns over catalog msgid field
# ---------------------------------------------------------------------------
log("\n=== C) Catalog lookup patterns (LDRH msgid + CMP + stride add) ===")
# Heuristic: functions that load [base+#2] (msgid) and add #0x1c
lookup_hits: list[int] = []
i = CODE_BAND[0]
while i < CODE_BAND[1] - 8:
    w = u16(i)
    # LDRH rt,[rn,#2] T1: 10001 imm5 rt rn  — imm=1 for halfword offset 2
    if (w & 0xF800) == 0x8800:
        imm = ((w >> 6) & 0x1F) << 1
        if imm == 2:
            # look ahead 0x40 for ADD #0x1c / MOVS #0x1c
            window = img[i : i + 0x50]
            if b"\x1c" in window[2:]:  # weak
                # stronger: MOVS rd,#0x1c = 0x21xx with imm 0x1c → 0x211c..0x271c
                for j in range(i, min(i + 0x50, len(img) - 2), 2):
                    ww = u16(j)
                    if (ww & 0xF800) == 0x2000 and (ww & 0xFF) == 0x1C:
                        lookup_hits.append(i)
                        break
                    # ADD rd, rn, #imm T1: 0001110 imm3 rn rd — imm=0x1c not possible (3-bit)
                    # ADD.W / ADDW with 0x1c
                    if (ww & 0xF800) >= 0xE800:
                        ww2 = u16(j + 2)
                        # ADDW: F2xx / F6? — check imm12 encoding for 0x1c
                        if (ww & 0xFBF0) == 0xF200:  # ADDW
                            rd, imm12 = dec_imm(ww, ww2)
                            if imm12 == 0x1C:
                                lookup_hits.append(i)
                                break
    if (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2

# Dedup nearby
lookup_hits = sorted(set(lookup_hits))
log(f"LDRH[+2]+stride0x1c candidates={len(lookup_hits)}")
for h in lookup_hits[:25]:
    pro = find_prolog(h)
    log(f"\n--- lookup cand @{hex(h)} prolog@{hex(pro)} va={hex(va_of(pro))} ---")
    for off, text in disasm_window(pro, 0x100)[:40]:
        log(f"  {hex(off)}: {text}")
    loads = collect_hdr_loads(pro, 0x180)
    if loads:
        imm_hist = Counter(x[2] for x in loads)
        log(f"  hdr-ish imm hist: {dict(sorted(imm_hist.items()))}")

# ---------------------------------------------------------------------------
# D) Nanobp / pb_* near OEM catalog / dispatcher path strings (names only as tags)
# ---------------------------------------------------------------------------
log("\n=== D) Nanopb / pb_encode/decode near OEM IPC islands ===")
nanopb_needles = [
    b"pb_decode",
    b"pb_encode",
    b"pb_encode_delimited",
    b"pb_decode_delimited",
    b"pb_istream",
    b"pb_ostream",
    b"nanopb",
    b"PB_BYTES_ARRAY",
    b"pb_field_t",
    b"pb_msgdesc",
]
for nd in nanopb_needles:
    pos = 0
    n = 0
    while n < 8:
        i = img.find(nd, pos)
        if i < 0:
            break
        near = []
        for p in range(max(0, i - 0x40), min(len(img), i + 0x60)):
            if 32 <= img[p] < 127:
                j = p
                while j < len(img) and 32 <= img[j] < 127:
                    j += 1
                if j - p >= 5:
                    near.append(img[p:j].decode())
                    p = j
                else:
                    p += 1
            else:
                p += 1
        oemish = [s for s in near if "OEM" in s or "IPC" in s or "SIM" in s or "sit" in s.lower()]
        log(f"  {nd!r} @{hex(i)} va={hex(va_of(i))} oemish={oemish[:6]}")
        n += 1
        pos = i + 1

# Descriptor tables: look for pb_field_t-like sequences near catalog (tag/type/offset arrays)
# nanopb classic: u8 tag, u8 type, u8 offset... or newer pb_msgdesc_t with field_info words
log("\n=== D2) Descriptor-ish tables near catalog / flags=2 body sizes ===")
# Scan 0x6d8000..0x6e2000 for sequences of (tag, type) patterns with type in nanopb set
# nanopb LTYPE: 0=bool,1=int32,... 6=bytes, 7=string, 8=submessage
desc_hits = []
for o in range(0x6D8000, 0x6E2000 - 16, 4):
    # look for word that equals a known msgid body size pattern: unlikely
    # Better: find ptrs into catalog bank from this island
    w = u32(o)
    if w in (va_of(0x6DE740), va_of(0x6DE724), va_of(0x6DE874)):
        desc_hits.append(o)
log(f"lit words == SIM catalog entry VAs in island: {len(desc_hits)} {[hex(x) for x in desc_hits[:20]]}")

# Scan for callback-table: pairs of (msgid_u16, fn_ptr) in OEM code/data
log("\n=== D3) msgid→fn table candidates in 0x6d0000..0x6f0000 ===")
msgid_fn: list[tuple[int, int, int]] = []
for o in range(0x6D0000, 0x6F0000 - 8, 4):
    mid = u16(o)
    pad = u16(o + 2)
    fn = u32(o + 4)
    if mid not in (0x2F50, 0x2F52, 0x2F57, 0x2F58):
        continue
    if pad not in (0, 1, 2, 4, 8, 0x101, 0x104):
        # allow any small pad
        if pad > 0x20 and pad not in (0x23,):
            continue
    if not (0x40000000 <= fn <= 0x46000000):
        continue
    # thumb bit often set
    fo = off_of(fn & ~1)
    if not (0 < fo < len(img)):
        continue
    msgid_fn.append((o, mid, fn))
log(f"msgid+fn candidates={len(msgid_fn)}")
for o, mid, fn in msgid_fn[:30]:
    log(f"  @{hex(o)} id={hex(mid)} fn={hex(fn)} file@{hex(off_of(fn & ~1))}")

# ---------------------------------------------------------------------------
# E) DBT file-path VAs → emit sites without string loads
#    Prior: dispatcher.c path VA 0x41067caa appears IN the DBT record itself.
#    Hunt code that loads DBT record address (not string) and branches.
# ---------------------------------------------------------------------------
log("\n=== E) DBT-record consumers (magic-adjacent code VAs) ===")
DBT_MAGIC = 0xFECDBA98
# Known log strings → find DBT records, extract line_no + file_va from record
needles = {
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "preprocess": b"[OEM][IPC] Failed to preprocess id %d",
    "invalid": b"[OEM][IPC] Invalid message",
    "decode": b"[OEM][IPC] Unable to decode message",
    "enc_size": b"[OEM][IPC] Unable to get encoded size",
    "encode": b"[OEM][IPC] Unable to encode IPC message",
    "sit_recv": b"[OEM][SIT] Received packet from channel %u, size %u",
}
dbt_recs: list[dict] = []
for name, nd in needles.items():
    so = img.find(nd)
    if so < 0:
        log(f"  {name}: MISSING")
        continue
    sva = va_of(so)
    # Find DBT where word+4 or nearby == sva
    found = None
    for o in range(max(0, (so - 0x100) & ~3), min(len(img) - 24, so + 0x40), 4):
        if u32(o) != DBT_MAGIC:
            continue
        words = [u32(o + 4 * k) for k in range(1, 6)]
        if sva in words:
            found = (o, words)
            break
    if not found:
        log(f"  {name}: no DBT near string @{hex(so)}")
        continue
    o, words = found
    # Typical: magic, fmt_va, line, file_va, ...
    fmt_va = words[0]
    line = words[1]
    file_va = words[2]
    dbt_recs.append(
        {
            "name": name,
            "dbt_off": o,
            "dbt_va": va_of(o),
            "fmt_va": fmt_va,
            "line": line,
            "file_va": file_va,
            "file": cstr(off_of(file_va), 80) if VA0 <= file_va < VA0 + 0x8000000 else None,
        }
    )
    log(
        f"  {name}: DBT@{hex(o)} va={hex(va_of(o))} line={hex(line) if isinstance(line,int) else line} "
        f"file={cstr(off_of(file_va),60)!r}"
    )

# Consumers of DBT *record* VA (not format string)
log("\n=== E2) MOVW/MOVT / litpool to DBT record VAs ===")
for rec in dbt_recs:
    tva = rec["dbt_va"]
    movs = find_movw_movt_va(tva, CODE_BAND, limit=20)
    lits = litpool_hits(tva, (0x6D0000, 0x6F0000), limit=20)
    lits2 = litpool_hits(tva, CODE_BAND, limit=20)
    log(f"  {rec['name']} dbt_va={hex(tva)} movw={len(movs)} {[hex(x) for x in movs[:8]]}")
    log(f"    lit_island={len(lits)} {[hex(x) for x in lits[:8]]} lit_codeband={len(lits2)} {[hex(x) for x in lits2[:8]]}")

# ---------------------------------------------------------------------------
# F) REQUEST type-check + msgid extract: scan for CMP #REQUEST-ish + LDRH clusters
#    "not a REQUEST" implies a type field check. Hunt CMP imm near catalog lookup.
# ---------------------------------------------------------------------------
log("\n=== F) REQUEST type field: CMP immediates near catalog / OEM island code ===")
# From litpool island 0x6dff00 — the DBT island is data; find CODE that references
# nearby absolute addresses used as jump tables.
# Strategy: find functions that LDRB/LDRH offset 0..16 then CMP #1/#2/#3/#4 (type enum)
type_check_hits: list[tuple[int, int, int]] = []  # (site, load_imm, cmp_imm)
i = 0x3000000  # focus higher MAIN where prior builders lived (~0x32e..)
# Also scan around known catalog xref sites from prior: 0x32e4494, 0x3388318
focus_bands = [
    (0x32E4000, 0x32E6000),
    (0x3388000, 0x338A000),
    (0x2CA8000, 0x2CAA000),  # BL target from builders
    (0x2D26000, 0x2D28000),
    (0x2E14000, 0x2E16000),
    (0x2400000, 0x2500000),
]
# Broader: any MOVW to catalog VA then nearby CMP
for name, tva in list(bank_vas.items())[:3]:
    for site in find_movw_movt_va(tva, (0x2000000, 0x3A00000), limit=40):
        for off, text in disasm_window(site, 0x80):
            if text.startswith("CMP ") and "#" in text:
                try:
                    cimm = int(text.split("#")[-1], 0)
                except ValueError:
                    continue
                if cimm in (0, 1, 2, 3, 4, 5, 8, 16):
                    type_check_hits.append((site, -1, cimm))
        loads = collect_hdr_loads(site - 0x40, 0xC0)
        for loff, ltext, limm in loads:
            type_check_hits.append((loff, limm, -1))

log(f"type/hdr touch near catalog MOVW sites: {len(type_check_hits)}")
# Summarize imm frequencies for loads
load_imms = Counter(x[1] for x in type_check_hits if x[1] >= 0)
cmp_imms = Counter(x[2] for x in type_check_hits if x[2] >= 0)
log(f"  load_imm hist: {dict(sorted(load_imms.items()))}")
log(f"  cmp_imm hist: {dict(sorted(cmp_imms.items()))}")

# ---------------------------------------------------------------------------
# G) Deep-dive: disasm every unique catalog litpool consumer + MOVW site
# ---------------------------------------------------------------------------
log("\n=== G) Deep disasm of catalog consumers ===")
consumer_sites: set[int] = set()
for name, tva in bank_vas.items():
    for lit in litpool_hits(tva, CODE_BAND, limit=50):
        # find nearby LDR that might load this literal — scan back 0x100 for LDR pc-rel
        for j in range(max(0, lit - 0x100), lit, 2):
            w = u16(j)
            if (w & 0xF800) == 0x4800:
                imm = (w & 0xFF) << 2
                target = ((j + 4) & ~2) + imm
                if abs(target - lit) <= 2:
                    consumer_sites.add(find_prolog(j))
            if (w & 0xF800) >= 0xE800:
                w2 = u16(j + 2)
                if w == 0xF8DF or (w & 0xFF7F) == 0xF85F:
                    imm = w2 & 0xFFF
                    u = (w >> 7) & 1
                    base = (j + 4) & ~3
                    target = base + imm if u else base - imm
                    if abs(target - lit) <= 2:
                        consumer_sites.add(find_prolog(j))
    for m in find_movw_movt_va(tva, CODE_BAND, limit=30):
        consumer_sites.add(find_prolog(m))

log(f"unique consumer prologs={len(consumer_sites)}")
for pro in sorted(consumer_sites)[:20]:
    log(f"\n--- consumer @{hex(pro)} va={hex(va_of(pro))} ---")
    for off, text in disasm_window(pro, 0x140)[:50]:
        log(f"  {hex(off)}: {text}")
    loads = collect_hdr_loads(pro, 0x180)
    if loads:
        log(f"  field touches: {[(hex(a), b, c) for a,b,c in loads[:25]]}")

# ---------------------------------------------------------------------------
# H) Wire-header hypothesis from preprocess sibling: scan dispatcher-ish
#    functions that take buffer ptr and do sequential LDRB/LDRH at 0,1,2,4,...
# ---------------------------------------------------------------------------
log("\n=== H) Sequential header-parse clusters (LDRB/H offs 0..16 monotonic) ===")
# Scan code for windows where ≥4 distinct small offsets are loaded from same rn
seq_hits: list[tuple[int, tuple[int, ...]]] = []
scan_lo, scan_hi = 0x2C00000, 0x3400000  # prior OEM builder neighborhood
i = scan_lo
while i < scan_hi - 0x40:
    loads_by_rn: dict[int, list[int]] = {}
    j = i
    end = i + 0x60
    while j < end:
        w = u16(j)
        if (w & 0xF800) == 0x7800:  # LDRB
            rt, rn, imm = w & 7, (w >> 3) & 7, (w >> 6) & 0x1F
            if imm <= 16:
                loads_by_rn.setdefault(rn, []).append(imm)
            j += 2
        elif (w & 0xF800) == 0x8800:  # LDRH
            rt, rn, imm = w & 7, (w >> 3) & 7, ((w >> 6) & 0x1F) << 1
            if imm <= 16:
                loads_by_rn.setdefault(rn, []).append(imm)
            j += 2
        elif (w & 0xF800) >= 0xE800:
            w2 = u16(j + 2)
            if (w & 0xFFF0) == 0xF890:  # LDRB.W
                rn, imm = w & 0xF, w2 & 0xFFF
                if imm <= 16:
                    loads_by_rn.setdefault(rn, []).append(imm)
            elif (w & 0xFFF0) == 0xF8B0:  # LDRH.W
                rn, imm = w & 0xF, w2 & 0xFFF
                if imm <= 16:
                    loads_by_rn.setdefault(rn, []).append(imm)
            j += 4
        else:
            j += 2
    for rn, imms in loads_by_rn.items():
        uniq = tuple(sorted(set(imms)))
        if len(uniq) >= 4 and uniq[0] <= 2:
            seq_hits.append((i, uniq))
    i += 0x20  # stride for speed

# Dedup by similar offset sets
best: dict[tuple[int, ...], int] = {}
for site, uniq in seq_hits:
    if uniq not in best or site < best[uniq]:
        best[uniq] = site
log(f"sequential hdr-parse patterns={len(best)}")
for uniq, site in sorted(best.items(), key=lambda x: (-len(x[0]), x[1]))[:30]:
    pro = find_prolog(site)
    log(f"\n  pattern offs={uniq} @{hex(site)} prolog@{hex(pro)} va={hex(va_of(pro))}")
    for off, text in disasm_window(pro, 0x100)[:35]:
        log(f"    {hex(off)}: {text}")

# Also scan around DBT file path code VAs if we can find emit stubs
log("\n=== I) Code near dispatcher.c / utils.c / sit_main.c path strings ===")
for path_sub in (
    b"oem_ipc_message_dispatcher.c",
    b"oem_ipc_message_utils.c",
    b"oem_sit_main.c",
):
    po = img.find(path_sub)
    if po < 0:
        log(f"  {path_sub!r}: missing")
        continue
    pva = va_of(po)
    # litpool refs to path VA — often in DBT only; also search full image lit
    lits = litpool_hits(pva, (0x6D0000, 0x6F0000), limit=20)
    log(f"  {path_sub.decode()} @{hex(po)} va={hex(pva)} island_lits={len(lits)}")
    for lit in lits[:6]:
        # dump neighboring words for function ptr tables
        words = [u32(lit + 4 * k) for k in range(-4, 8)]
        decoded = []
        for w in words:
            if VA0 <= w < VA0 + 0x8000000:
                s = cstr(off_of(w), 40)
                decoded.append(s if s else f"code?{hex(w)}")
            else:
                decoded.append(hex(w))
        log(f"    lit@{hex(lit)} neigh={decoded}")

# ---------------------------------------------------------------------------
# J) Body for flags=2: compare encode-size paths / fixed size immediates
# ---------------------------------------------------------------------------
log("\n=== J) Fixed size #2 / #12 / #14 immediates near catalog consumers ===")
# If app hdr is 12B (EXYNOS sibling) + 2B body = 14, or hdr includes body
size_hits = []
for pro in sorted(consumer_sites)[:30]:
    for off, text in disasm_window(pro, 0x180):
        if text.startswith("MOVS ") and "#" in text:
            try:
                imm = int(text.split("#")[-1], 0)
            except ValueError:
                continue
            if imm in (2, 4, 8, 10, 12, 14, 16, 20, 24, 28):
                size_hits.append((pro, off, text))
log(f"size-imm near consumers: {len(size_hits)}")
for pro, off, text in size_hits[:40]:
    log(f"  pro@{hex(pro)} {hex(off)}: {text}")

# ---------------------------------------------------------------------------
# K) Verdict synthesis
# ---------------------------------------------------------------------------
log("\n=== K) VERDICT (evidence-only) ===")
init = next((e for e in entries if e["msgid"] == 0x2F50), None)
if init:
    log(
        f"SIM_INIT_REQ catalog: body_hint={init['body']} meta={hex(init['meta'])} "
        f"rsp={hex(init['rsp'])} +18={hex(init['w18'])}"
    )
log(f"catalog consumers found: {len(consumer_sites)}")
log(f"lookup stride candidates: {len(lookup_hits)}")
log(f"msgid→fn table candidates: {len(msgid_fn)}")
log(f"sequential hdr patterns: {len(best)}")
# Frame recovered only if we have proven field map
log("FRAME_RECOVERED=NO (unless section G/H shows proven wire map — review above)")
log("Do not invent bytes. No soft SIM_INIT without proven header+body.")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"\nWrote {OUT}")
