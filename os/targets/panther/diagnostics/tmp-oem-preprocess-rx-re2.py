#!/usr/bin/env python3
"""Follow oem_sit_main RX → dispatcher preprocess; recover app header stores.

Uses DBT file attribution (oem_sit_main.c / oem_ipc_message_dispatcher.c)
and catalog-bank builders. Evidence only.
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


def find_prolog(addr, back=0x400):
    for j in range(addr, max(0, addr - back), -2):
        w = u16(j)
        if (w & 0xFF00) == 0xB500 or (w & 0xFE00) == 0xB400:
            return j
        if w == 0xE92D:
            return j
    return max(0, addr - 0x40)


def bl_target(i):
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


def disasm(start, length=0x280):
    out = []
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
                if imm <= 0x120:
                    out.append((i, f"{op} r{rt},[r{rn},#{imm}]"))
            elif (w & 0xF800) == 0xF000 and (w2 & 0xD000) == 0xD000:
                tgt = bl_target(i)
                fo = off_of(tgt) if tgt and VA0 <= tgt < VA0 + 0x8000000 else None
                out.append((i, f"BL ->{hex(tgt) if tgt else '?'} file@{hex(fo) if fo else '?'}"))
            i += 4
        else:
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                s = cstr(off_of(val)) if VA0 <= val < VA0 + 0x8000000 else None
                out.append((i, f"LDR r{rt},[pc] lit@{hex(lit)}={hex(val)} {s!r}"))
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


def scan_movw_movt(va: int, lo: int = 0, hi: int | None = None) -> list[int]:
    if hi is None:
        hi = len(img) - 8
    want_lo, want_hi = va & 0xFFFF, (va >> 16) & 0xFFFF
    hits = []
    i = lo & ~1
    while i < hi:
        w, w2 = u16(i), u16(i + 2)
        if (w & 0xFBF0) == 0xF240:
            rd, imm = dec_imm(w, w2)
            if imm == want_lo:
                j = i + 4
                while j < i + 24:
                    ww, ww2 = u16(j), u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0:
                        rd2, imm2 = dec_imm(ww, ww2)
                        if rd2 == rd and imm2 == want_hi:
                            hits.append(i)
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


def find_ldr_pc_to(lit_off: int, code_lo: int, code_hi: int) -> list[tuple[int, int]]:
    hits = []
    i = code_lo & ~1
    while i < code_hi - 4:
        w, w2 = u16(i), u16(i + 2)
        # LDR T1
        if (w & 0xF800) == 0x4800:
            imm = (w & 0xFF) << 2
            for align in (2, 3):
                lit = ((i + 4) & ~align) + imm if align == 2 else ((i + 4) & ~3) + imm
                if lit == lit_off or abs(lit - lit_off) <= 2:
                    hits.append((i, (w >> 8) & 7))
                    break
            i += 2
            continue
        # LDR.W [pc]
        if (w & 0xFF7F) == 0xF85F:
            imm = w2 & 0xFFF
            u = (w >> 7) & 1
            base = (i + 4) & ~3
            lit = base + imm if u else base - imm
            if lit == lit_off or abs(lit - lit_off) <= 2:
                hits.append((i, (w2 >> 12) & 0xF))
            i += 4
            continue
        if (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    return hits


log(f"MAIN size={len(img)}")

# --- 1) Path strings ---
disp = img.find(b"oem_ipc_message_dispatcher.c")
utils = img.find(b"oem_ipc_message_utils.c")
sitm = img.find(b"oem_sit_main.c")
log(f"dispatcher.c @{hex(disp)} va={hex(va_of(disp))}")
log(f"utils.c @{hex(utils)} va={hex(va_of(utils))}")
log(f"oem_sit_main.c @{hex(sitm)} va={hex(va_of(sitm))}")

# DBT line nums for preprocess cluster (from prior):
# not_req line=0x51, msgid_nf=0x57, preprocess=0x62, all file=dispatcher@0x41067caa
# invalid line=0x95 file=0x41067cff
# sit_recv line=0xda file=sit_main@0x41068655

# --- 2) Find litpools that point at sit_recv DBT record or sit path; then LDR from code ---
sit_recv_str = img.find(b"[OEM][SIT] Received packet from channel %u, size %u")
sit_recv_va = va_of(sit_recv_str)
msgid_nf_va = va_of(img.find(b"[OEM][IPC] Message ID not found"))
prep_va = va_of(img.find(b"[OEM][IPC] Failed to preprocess id %d"))
not_req_va = va_of(img.find(b"[OEM][IPC] Message is not a REQUEST"))

log(f"\nsit_recv_va={hex(sit_recv_va)} msgid_nf={hex(msgid_nf_va)} prep={hex(prep_va)}")

# Find ALL litpool occurrences of sit path VA and dispatcher path VA used in DBT
sit_path_va = va_of(sitm)
disp_path_va = va_of(disp)  # may differ from truncated path in DBT
# DBT used 0x41068655 and 0x41067caa — find those exact VAs in image as path starts
log(f"sit_path_va={hex(sit_path_va)} disp_path_va={hex(disp_path_va)}")
# The DBT file field pointed mid-string; recover full path starts
for label, va in [("sit_dbt_file", 0x41068655), ("disp_dbt_file", 0x41067CAA), ("disp2", 0x41067CFF)]:
    o = off_of(va)
    log(f"  {label} @{hex(o)}: {cstr(o, 90)!r}")

# --- 3) Hunt DBT emit: many Shannon builds load record ptr then BL log ---
# Search for MOVW+MOVT assembling DBT record VAs (e.g. va_of(0x6dff18))
dbt_recs = {
    "not_req_dbt": va_of(0x6DFEFC),
    "msgid_nf_dbt": va_of(0x6DFF18),
    "prep_dbt": va_of(0x6DFF34),
    "invalid_dbt": va_of(0x6DFF6C),
    "sit_recv_dbt": va_of(0x6DFFE0),
}
log("\n=== MOVW+MOVT to DBT record VAs (full image) ===")
for name, va in dbt_recs.items():
    hits = scan_movw_movt(va)
    log(f"{name} va={hex(va)}: n={len(hits)} {[hex(h) for h in hits[:8]]}")
    for h in hits[:2]:
        prolog = find_prolog(h)
        log(f"  fn~{hex(prolog)} va={hex(va_of(prolog))}")
        for off, txt in disasm(prolog, 0x200):
            if any(k in txt for k in ("LDR", "STR", "MOVW", "MOVT", "CMP", "MOVS", "BL ", "PUSH")):
                log(f"    {hex(off)}: {txt}")

# --- 4) Alternate: LDR.W to lit containing string VA (direct, rare) or to DBT magic neighborhood ---
log("\n=== LDR pc-rel to string VAs / DBT magics in code bands ===")
# Focus bands where Thumb code lives densely: 0x3000000..0x3400000 and 0x1000000..0x2000000
bands = [(0x100000, 0x2800000), (0x3000000, 0x3C00000)]
for sname, sva in [("msgid_nf", msgid_nf_va), ("prep", prep_va), ("sit_recv", sit_recv_va), ("not_req", not_req_va)]:
    # litpool sites
    t = struct.pack("<I", sva)
    lits = []
    pos = 0
    while len(lits) < 12:
        i = img.find(t, pos)
        if i < 0:
            break
        lits.append(i)
        pos = i + 4
    log(f"{sname} lit sites={[hex(x) for x in lits]}")
    for lit in lits:
        # skip if inside OEM string/DBT island (0x6dxxxx) — those aren't code litpools
        if 0x6D0000 <= lit <= 0x6E8000:
            continue
        for lo, hi in bands:
            hits = find_ldr_pc_to(lit, lo, hi)
            if hits:
                log(f"  lit@{hex(lit)} LDR hits={[hex(h[0]) for h in hits[:6]]}")
                for h, _rt in hits[:2]:
                    prolog = find_prolog(h)
                    log(f"    fn~{hex(prolog)}")
                    for off, txt in disasm(prolog, 0x1C0):
                        if any(k in txt for k in ("LDR", "STR", "MOVW", "CMP", "MOVS", "BL ")):
                            log(f"      {hex(off)}: {txt}")

# --- 5) Deep-dive catalog bank builder @0x3388318 (prior hit) ---
log("\n=== Catalog bank builder @0x3388318 full field stores ===")
for off, txt in disasm(0x3388318, 0x300):
    if any(k in txt for k in ("STR", "LDR", "MOVW", "MOVT", "MOVS", "CMP", "BL ")):
        log(f"  {hex(off)}: {txt}")

# --- 6) Catalog SIM_INIT entry consumer @0x32e4494 ---
log("\n=== cat_init consumer around 0x32e4494 ===")
# dump wider from prolog
prolog = find_prolog(0x32E4494, 0x800)
log(f"prolog={hex(prolog)}")
# find the exact MOVW site and dump ±0x100 with focus on loads/stores
for off, txt in disasm(0x32E4400, 0x200):
    if any(k in txt for k in ("STR", "LDR", "MOVW", "MOVT", "MOVS", "CMP", "BL ")):
        log(f"  {hex(off)}: {txt}")

# --- 7) Hunt functions that LDRH catalog body (offset 0) and CMP against length ---
# Pattern: load entry pointer, LDRH [entry,#0] as expected body size
log("\n=== Hunt LDRH [rn,#0] + LDRH [rn,#2] within 16B (catalog entry read) ===")
pair_sites = []
i = 0x200000
while i < 0x3C00000 and len(pair_sites) < 40:
    w = u16(i)
    if (w & 0xF800) == 0x8800:  # LDRH
        imm = ((w >> 6) & 0x1F) << 1
        rn = (w >> 3) & 7
        if imm == 0:
            # look ahead 16B for LDRH same rn #2
            for j in range(i + 2, i + 18, 2):
                w2 = u16(j)
                if (w2 & 0xF800) == 0x8800:
                    imm2 = ((w2 >> 6) & 0x1F) << 1
                    rn2 = (w2 >> 3) & 7
                    if rn2 == rn and imm2 == 2:
                        pair_sites.append((i, j, rn))
                        break
                if (w2 & 0xF800) >= 0xE800:
                    # also check LDRH.W
                    pass
    if (w & 0xF800) >= 0xE800:
        i += 4
    else:
        i += 2

log(f"LDRH#0 + LDRH#2 same rn pairs: {len(pair_sites)}")
for a, b, rn in pair_sites[:15]:
    # check if nearby has MOVW 0x1c (stride) or CMP with body
    window = img[max(0, a - 0x40) : a + 0x80]
    has_1c = b"\x1c" in window  # weak
    # disasm snippet
    interesting = False
    snippets = []
    for off, txt in disasm(a - 0x20, 0x80):
        snippets.append(f"{hex(off)}:{txt}")
        if "0x2f" in txt or "CMP" in txt or "0x1c" in txt or "MOVW" in txt:
            interesting = True
    if interesting or True:
        # filter: must have MOVW catalog-ish or CMP # small
        text = " ".join(snippets)
        if any(x in text for x in ("0x2f", "0x1c", "0x6d", "0x406d")) or "CMP r" in text:
            log(f"\n  pair@{hex(a)}/{hex(b)} r{rn}")
            for s in snippets:
                log(f"    {s}")

# --- 8) libsitril 0x2f50 record interpretation ---
libp = resolve("Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so")
if libp.exists():
    lib = libp.read_bytes()
    log("\n=== libsitril registry around 0x2f50 ===")
    o = 0x95660
    for i in range(-2, 6):
        p = o + i * 24
        chunk = lib[p : p + 24]
        mid = struct.unpack_from("<H", chunk, 0)[0]
        b2 = chunk[2]
        b3 = chunk[3]
        u32_4 = struct.unpack_from("<I", chunk, 4)[0]
        u32_8 = struct.unpack_from("<I", chunk, 8)[0]
        u32_12 = struct.unpack_from("<I", chunk, 12)[0]
        log(
            f"  @{hex(p)} mid={hex(mid)} b2={hex(b2)} b3={hex(b3)} "
            f"+4={hex(u32_4)} +8={hex(u32_8)} +12={hex(u32_12)} "
            f"raw={' '.join(f'{x:02x}' for x in chunk)}"
        )

# --- 9) meta 0x10104 decode across catalog: collect unique +18/+14 fields ---
log("\n=== Catalog meta/tail histogram (first 80 OEM entries from 0x6de000-ish) ===")
# find start of OEM catalog: walk back from SIM_INFO
start = 0x6DE000
# dump entries where msgid in 0x2f00..0x2fff
count = 0
for o in range(0x6D8000, 0x6E0000, 4):
    mid = u16(o + 2)
    body = u16(o)
    if 0x2F00 <= mid <= 0x2FFF and body < 0x2000:
        name_va = u32(o + 4)
        name = cstr(off_of(name_va), 36) if VA0 <= name_va < VA0 + 0x8000000 else None
        if name and name.startswith("SIM_"):
            meta = u32(o + 8)
            rsp = u16(o + 0xC)
            w14 = u32(o + 0x14)
            w18 = u32(o + 0x18)
            log(
                f"  @{hex(o)} body={body} id={hex(mid)} meta={hex(meta)} rsp={hex(rsp)} "
                f"+14={hex(w14)} +18={hex(w18)} {name}"
            )
            count += 1
            if count >= 25:
                break

# --- 10) Verdict ---
log("\n=== VERDICT ===")
log("DBT: msgid_nf/preprocess/not_req → oem_ipc_message_dispatcher.c (path VA only)")
log("DBT: sit_recv → oem_sit_main.c")
log("preprocess_cb strings = gmetrics only (false friend)")
log("App header field order/size from RX preprocess: still unrecovered if no DBT-emit code xref")
log("DONE")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"Wrote {OUT}", flush=True)
