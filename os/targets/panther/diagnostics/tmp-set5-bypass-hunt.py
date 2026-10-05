#!/usr/bin/env python3
"""Hunt STATUS READY / SET#5 without Present==2 on MAIN B.

Looks for:
  - alternate STATUS entries / different CMP toward SET#5
  - non-STRB / aliased Present(+0) writes (STRH/STR/STM/memcpy)
  - wrong-object theory (STRB=2 into other getobj bases)
  - SET_APP callers that skip Present check
  - LDRB of Present via non-+0xBF6 aliases into READY path

Image: saaios-probe-b-modem.bin (live B / 449eeab3…).
"""
from __future__ import annotations

import hashlib
import struct
from collections import defaultdict
from pathlib import Path

CANDIDATES = [
    Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"),
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path(r"C:\Users\Admin\Projects\saaios-som\fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]

SET_APP = 0x19916D2
STATUS_PROLOG = 0x14FB322
READY_SITE = 0x14FB5C6
BF6_STRB = 0x14FB380
FN_A = 0x14F6A16  # Present=2 STRB site (approx latch)
PRESENT_CTOR = 0x1A552A4
STATUS_WRAP = 0x14C6626
GETOBJ = None  # filled if found


def load_img() -> tuple[Path, memoryview]:
    for p in CANDIDATES:
        if p.is_file():
            raw = p.read_bytes()
            return p, memoryview(raw)
    raise SystemExit("MAIN image not found in candidates")


def u16(img: memoryview, off: int) -> int:
    return struct.unpack_from("<H", img, off)[0]


def u32(img: memoryview, off: int) -> int:
    return struct.unpack_from("<I", img, off)[0]


def bl_target(pc: int, h1: int, h2: int) -> int | None:
    if (h1 & 0xF800) != 0xF000:
        return None
    if (h2 & 0xD000) != 0xD000:
        return None
    s = (h1 >> 10) & 1
    imm10 = h1 & 0x3FF
    j1 = (h2 >> 13) & 1
    j2 = (h2 >> 11) & 1
    imm11 = h2 & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return (pc + 4 + imm32) & 0xFFFFFFFF


def find_bl(img: memoryview, tgt: int, start: int = 0x100000, end: int | None = None) -> list[int]:
    t = tgt & ~1
    end = min(end or 0x1C00000, len(img) - 4)
    hits = []
    o = start
    while o < end:
        h1 = u16(img, o)
        if (h1 & 0xF800) == 0xF000:
            h2 = u16(img, o + 2)
            bt = bl_target(o, h1, h2)
            if bt is not None and (bt & ~1) == t:
                hits.append(o)
                o += 4
                continue
        o += 2
    return hits


def preceding_movs_r0(img: memoryview, site: int, lookback: int = 16) -> int | None:
    for back in range(2, lookback + 2, 2):
        hh = u16(img, site - back)
        if (hh & 0xF800) == 0x2000 and ((hh >> 8) & 7) == 0:
            return hh & 0xFF
    return None


def scan_ldrb_imm12(img: memoryview, imm: int) -> list[tuple[int, int, int]]:
    """LDRB.W Rt,[Rn,#imm12] → (off, rt, rn)."""
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF890:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == imm:
            hits.append((o, (h2 >> 12) & 0xF, h & 0xF))
    return hits


def scan_strb_imm12(img: memoryview, imm: int) -> list[tuple[int, int, int]]:
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF880:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == imm:
            hits.append((o, (h2 >> 12) & 0xF, h & 0xF))
    return hits


def scan_strh_imm12(img: memoryview, imm: int) -> list[tuple[int, int, int]]:
    """STRH.W Rt,[Rn,#imm12] encoding T2: F8An tttt iiii iiii iiii"""
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF8A0:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == imm:
            hits.append((o, (h2 >> 12) & 0xF, h & 0xF))
    return hits


def scan_str_imm12(img: memoryview, imm: int) -> list[tuple[int, int, int]]:
    """STR.W Rt,[Rn,#imm12] encoding T3: F8Cn"""
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF8C0:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == imm:
            hits.append((o, (h2 >> 12) & 0xF, h & 0xF))
    return hits


def scan_strb_reg_imm5(img: memoryview, imm5: int) -> list[tuple[int, int, int]]:
    """16-bit STRB Rt,[Rn,#imm5] — used for PresentObj[+0] when base in lo regs."""
    hits = []
    end = min(len(img) - 2, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xF800) != 0x7000:
            continue
        if ((h >> 6) & 0x1F) != imm5:
            continue
        hits.append((o, h & 7, (h >> 3) & 7))
    return hits


def scan_movs_imm_near_strb2(img: memoryview, window: int = 0x20) -> list[tuple[int, int, int]]:
    """Sites: MOVS Rd,#2 then nearby STRB Rd,[Rn,#0] within window (both dirs).

    Classic Present=2 latch pattern (FN_A and similar).
    """
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xF800) != 0x2000:
            continue
        if (h & 0xFF) != 2:
            continue
        rd = (h >> 8) & 7
        # look forward for STRB rd,[rn,#0] (16-bit or STRB.W #0)
        for fwd in range(2, window + 2, 2):
            p = o + fwd
            if p + 2 >= end:
                break
            hh = u16(img, p)
            # STRB Rt,[Rn,#imm5=0]
            if (hh & 0xF800) == 0x7000 and ((hh >> 6) & 0x1F) == 0 and (hh & 7) == rd:
                hits.append((o, p, (hh >> 3) & 7))
                break
            # STRB.W Rt,[Rn,#0]
            if (hh & 0xFFF0) == 0xF880 and p + 4 <= end:
                h2 = u16(img, p + 2)
                if ((h2 >> 12) & 0xF) == rd and (h2 & 0xFFF) == 0:
                    hits.append((o, p, hh & 0xF))
                    break
    return hits


def decode_lite(img: memoryview, start: int, end: int) -> list[tuple[int, str]]:
    out: list[tuple[int, str]] = []
    o = start
    while o < end:
        h = u16(img, o)
        if (h & 0xE000) == 0xE000 and (h & 0x1800) != 0:
            if o + 2 >= end:
                break
            h2 = u16(img, o + 2)
            tgt = bl_target(o, h, h2)
            if tgt is not None:
                tag = "SET_APP" if (tgt | 1) == (SET_APP | 1) else ""
                out.append((o, f"BL 0x{tgt:x} {tag}".rstrip()))
                o += 4
                continue
            if (h & 0xFFF0) == 0xF890:
                out.append((o, f"LDRB.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]"))
                o += 4
                continue
            if (h & 0xFFF0) == 0xF880:
                out.append((o, f"STRB.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]"))
                o += 4
                continue
            if (h & 0xFFF0) == 0xF8A0:
                out.append((o, f"STRH.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]"))
                o += 4
                continue
            if (h & 0xFFF0) == 0xF8C0:
                out.append((o, f"STR.W r{(h2>>12)&0xF},[r{h&0xF},#0x{h2&0xFFF:x}]"))
                o += 4
                continue
            # STMDB / STM (T1 32-bit): E88x / E90x rough
            if (h & 0xFFD0) == 0xE880 or (h & 0xFFD0) == 0xE900:
                out.append((o, f"STM/LDM.W {h:04x}{h2:04x}"))
                o += 4
                continue
            # MOVW / MOVT
            if (h & 0xFBF0) == 0xF240 or (h & 0xFBF0) == 0xF2C0:
                out.append((o, f"MOVW/T {h:04x}{h2:04x}"))
                o += 4
                continue
            out.append((o, f"t32 {h:04x}{h2:04x}"))
            o += 4
            continue
        if (h & 0xF800) == 0x2000:
            out.append((o, f"MOVS r{(h>>8)&7},#{h&0xFF}"))
            o += 2
            continue
        if (h & 0xF800) == 0x2800:
            out.append((o, f"CMP r{(h>>8)&7},#{h&0xFF}"))
            o += 2
            continue
        if (h & 0xFFC0) == 0x4280:
            out.append((o, f"CMP r{h&7},r{(h>>3)&7}"))
            o += 2
            continue
        if (h & 0xF500) == 0xB100:
            op = "CBNZ" if (h & 0x0800) else "CBZ"
            out.append((o, f"{op} r{h&7}"))
            o += 2
            continue
        if (h & 0xF000) == 0xD000 and (h & 0x0F00) not in (0x0E00, 0x0F00):
            conds = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
            imm8 = h & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            dest = o + 4 + (imm8 << 1)
            out.append((o, f"B{conds[(h>>8)&0xF]} 0x{dest:x}"))
            o += 2
            continue
        if (h & 0xF800) == 0x7800:
            out.append((o, f"LDRB r{h&7},[r{(h>>3)&7},#{(h>>6)&0x1F}]"))
            o += 2
            continue
        if (h & 0xF800) == 0x7000:
            out.append((o, f"STRB r{h&7},[r{(h>>3)&7},#{(h>>6)&0x1F}]"))
            o += 2
            continue
        if h == 0x4770:
            out.append((o, "BX lr"))
            o += 2
            continue
        out.append((o, f"h16 {h:04x}"))
        o += 2
    return out


def find_memcpy_xrefs(img: memoryview) -> list[int]:
    """Locate memcpy-like by string or common PLT stubs — best-effort via 'memcpy' cstr xrefs."""
    raw = bytes(img)
    needle = b"memcpy\0"
    offs = []
    start = 0
    while True:
        i = raw.find(needle, start)
        if i < 0:
            break
        offs.append(i)
        start = i + 1
    return offs


def scan_status_alt_entries(img: memoryview) -> None:
    """Find every BL that lands inside STATUS cluster / STATUS_WRAP callers."""
    print("\n=== 1) STATUS_WRAP callers (expect 2: SADR + L1TUNNEL) ===")
    for o in find_bl(img, STATUS_WRAP):
        print(f"  BL@0x{o:x} → STATUS_WRAP")

    print("\n=== 2) Direct BL into STATUS prolog region 0x14fb322..0x14fb700 ===")
    # Scan for BL whose target falls in STATUS body
    end = min(len(img) - 4, 0x1C00000)
    hits = []
    o = 0x100000
    while o < end:
        h1 = u16(img, o)
        if (h1 & 0xF800) == 0xF000:
            h2 = u16(img, o + 2)
            bt = bl_target(o, h1, h2)
            if bt is not None and 0x14FB322 <= (bt & ~1) <= 0x14FB700:
                hits.append((o, bt & ~1))
                o += 4
                continue
        o += 2
    for site, tgt in hits:
        print(f"  BL@0x{site:x} → 0x{tgt:x}")
    print(f"  count={len(hits)}")


def analyze_set5(img: memoryview) -> list[tuple[int, int | None]]:
    print("\n=== 3) All BL → SET_APP with MOVS r0,#imm ===")
    hits_sites = find_bl(img, SET_APP)
    rows = []
    by_imm: dict[int, list[int]] = defaultdict(list)
    for site in hits_sites:
        imm = preceding_movs_r0(img, site)
        rows.append((site, imm))
        by_imm[imm if imm is not None else -1].append(site)
        print(f"  BL@0x{site:x}  MOVS r0,#{imm}")
    print(f"total_BL={len(rows)}")
    for imm in sorted(by_imm):
        sites = ", ".join(f"0x{s:x}" for s in by_imm[imm])
        print(f"  imm#{imm}: {len(by_imm[imm])} → {sites}")
    return rows


def analyze_set5_gates(img: memoryview, rows: list[tuple[int, int | None]]) -> None:
    print("\n=== 4) SET#5 gate audit (CMP immediates in 0x40 before BL) ===")
    anomalies = []
    for site, imm in rows:
        if imm != 5:
            continue
        win = decode_lite(img, site - 0x40, site + 4)
        cmps = []
        ldrbs = []
        for off, s in win:
            if s.startswith("CMP"):
                cmps.append((off, s))
            if "LDRB" in s:
                ldrbs.append((off, s))
        print(f"-- #5 @0x{site:x} --")
        for off, s in win:
            if any(k in s for k in ("LDRB", "CMP", "BEQ", "BNE", "CBZ", "CBNZ", "MOVS", "BL ")):
                print(f"  0x{off:x}: {s}")
        # last CMP imm
        last_imm = None
        for _, s in cmps:
            if "#" in s:
                try:
                    last_imm = int(s.split("#", 1)[1])
                except ValueError:
                    pass
        has_bf6 = any("0xbf6" in s.lower() for _, s in ldrbs)
        if last_imm != 2 or not has_bf6:
            anomalies.append((site, last_imm, has_bf6, cmps, ldrbs))
            print(f"  ** ANOMALY last_cmp={last_imm} has_bf6={has_bf6}")
        else:
            print(f"  OK: Present(+0xBF6)==2 gate")
    if not anomalies:
        print("No SET#5 bypass: every #5 has LDRB +0xBF6 then CMP #2")
    else:
        print(f"ANOMALY_COUNT={len(anomalies)}")


def analyze_bf6_stores(img: memoryview) -> None:
    print("\n=== 5) +0xBF6 store inventory (STRB/STRH/STR) ===")
    for label, fn in (
        ("STRB.W", scan_strb_imm12),
        ("STRH.W", scan_strh_imm12),
        ("STR.W", scan_str_imm12),
    ):
        hits = fn(img, 0xBF6)
        print(f"  {label} +0xBF6: count={len(hits)}")
        for o, rt, rn in hits:
            # context
            ctx = decode_lite(img, max(0, o - 0x10), o + 0x10)
            nearby = "; ".join(f"{s}" for _, s in ctx if "STR" in s or "LDR" in s or "MOV" in s)
            print(f"    @0x{o:x} rt={rt} rn={rn}  ctx≈[{nearby}]")


def analyze_present_obj_writes(img: memoryview) -> None:
    print("\n=== 6) MOVS #2 → STRB [Rn,#0] (Present=2 latch pattern) ===")
    hits = scan_movs_imm_near_strb2(img, window=0x28)
    print(f"  pattern_count={len(hits)}")
    # Dedup by STRB site; show unique STRB sites with nearby string hints
    by_strb: dict[int, int] = {}
    for mov, strb, rn in hits:
        by_strb.setdefault(strb, mov)
    for strb, mov in sorted(by_strb.items()):
        # look for nearby ASCII in ±0x80 via MOVW/MOVT — skip heavy; print window
        win = decode_lite(img, mov - 4, strb + 8)
        interesting = [s for _, s in win if any(k in s for k in ("MOVS", "STRB", "BL ", "CMP"))]
        print(f"  MOVS@0x{mov:x} STRB@0x{strb:x}: {interesting}")

    print("\n=== 7) STRB.W [Rn,#0] preceded by MOVS #2 within 0x30 (wide form) ===")
    # Already covered in scan_movs; also find STRB.W #0 with value 2 from MOVS hi-reg?
    end = min(len(img) - 4, 0x1C00000)
    count = 0
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF880:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) != 0:
            continue
        rt = (h2 >> 12) & 0xF
        # look back for MOVS rt,#2 (only lo regs 0-7)
        if rt > 7:
            continue
        found = False
        for back in range(2, 0x30, 2):
            hh = u16(img, o - back)
            if (hh & 0xF800) == 0x2000 and ((hh >> 8) & 7) == rt and (hh & 0xFF) == 2:
                found = True
                print(f"  STRB.W#0 @0x{o:x} from MOVS@0x{o-back:x} rn={h&0xF}")
                count += 1
                break
        if count > 40:
            print("  ... truncated")
            break
    print(f"  wide_strb0_from_movs2_shown={min(count,40)}")


def analyze_wrong_object(img: memoryview) -> None:
    print("\n=== 8) Wrong-object: STRB=2 into getobj(#0x10) / non-Present (known false friend) ===")
    # Prior: 0x146ac48 STRB=2 on getobj#0x10 — confirm still present; find peers
    # Scan MOVS #2 + STRB near BL getobj with arg 0x10 or 0x636c if we can find getobj
    # Heuristic: MOVW Rd,#0x636c / #0x10 near MOVS#2 STRB
    end = min(len(img) - 8, 0x1C00000)
    movw_636c = []
    movw_10 = []
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFBF0) != 0xF240:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0x8000) != 0:
            continue
        i = (h >> 10) & 1
        imm16 = ((h & 0xF) << 12) | (i << 11) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
        if imm16 == 0x636C:
            movw_636c.append(o)
        elif imm16 == 0x10:
            movw_10.append(o)
    print(f"  MOVW #0x636c sites={len(movw_636c)} (PresentObj id)")
    print(f"  MOVW #0x10 sites={len(movw_10)} (L1LC timer id — not Present)")
    # For each 0x636c: is there MOVS#2 STRB within ±0x80?
    present_writers = []
    for o in movw_636c:
        lo, hi = max(0x100000, o - 0x80), min(end, o + 0x80)
        for p in range(lo, hi, 2):
            hh = u16(img, p)
            if (hh & 0xF800) == 0x2000 and (hh & 0xFF) == 2:
                # STRB nearby
                for q in range(p, min(hi, p + 0x20), 2):
                    hq = u16(img, q)
                    if (hq & 0xF800) == 0x7000 and ((hq >> 6) & 0x1F) == 0:
                        present_writers.append((o, p, q))
                        break
                    if (hq & 0xFFF0) == 0xF880 and q + 2 < hi:
                        hq2 = u16(img, q + 2)
                        if (hq2 & 0xFFF) == 0:
                            present_writers.append((o, p, q))
                            break
    print(f"  PresentObj(#636c) near MOVS#2+STRB#0: {len(present_writers)}")
    for a, b, c in present_writers[:20]:
        print(f"    MOVW636c@0x{a:x} MOVS2@0x{b:x} STRB@0x{c:x}")

    # Same for #0x10 — expected false friends
    false = []
    for o in movw_10:
        lo, hi = max(0x100000, o - 0x40), min(end, o + 0x40)
        for p in range(lo, hi, 2):
            hh = u16(img, p)
            if (hh & 0xF800) == 0x2000 and (hh & 0xFF) == 2:
                for q in range(p, min(hi, p + 0x18), 2):
                    hq = u16(img, q)
                    if (hq & 0xF800) == 0x7000 and ((hq >> 6) & 0x1F) == 0:
                        false.append((o, p, q))
                        break
    print(f"  getobj(#0x10) near MOVS#2+STRB#0 (false friends): {len(false)}")
    for a, b, c in false[:10]:
        print(f"    MOVW10@0x{a:x} MOVS2@0x{b:x} STRB@0x{c:x}")


def analyze_memcpy_into_present(img: memoryview) -> None:
    print("\n=== 9) memcpy / wide copy into Present trio (+0xBF4..BF6) ===")
    # STATUS has STRB trio — look for STM that stores 3 bytes / STR of word covering BF4
    for imm in (0xBF4, 0xBF5, 0xBF6):
        str_w = scan_str_imm12(img, imm)
        strh = scan_strh_imm12(img, imm)
        print(f"  STR.W +0x{imm:x}: {len(str_w)}  STRH.W: {len(strh)}")
        for o, rt, rn in str_w[:8]:
            print(f"    STR.W @0x{o:x}")
        for o, rt, rn in strh[:8]:
            print(f"    STRH.W @0x{o:x}")

    # STM near STATUS copy site
    print("\n  STM/LDM near STATUS trio copy (0x14fb360..0x14fb3a0):")
    for off, s in decode_lite(img, 0x14FB360, 0x14FB3A0):
        print(f"    0x{off:x}: {s}")

    cstrs = find_memcpy_xrefs(img)
    print(f"\n  'memcpy\\0' cstr count={len(cstrs)} (informational; xrefs not fully chased)")


def analyze_alt_cmp(img: memoryview) -> None:
    print("\n=== 10) LDRB +0xBF6 consumers — all CMP immediates ===")
    for o, rt, rn in scan_ldrb_imm12(img, 0xBF6):
        win = decode_lite(img, o, o + 0x28)
        cmps = [s for _, s in win if s.startswith("CMP")]
        bls = [s for _, s in win if s.startswith("BL ")]
        print(f"  LDRB@0x{o:x} r{rt}←[r{rn}] CMPs={cmps} BLs={bls}")


def analyze_pin_skip_head(img: memoryview) -> None:
    print("\n=== 11) STATUS head: GET_APP∈{1,4} skip / Present re-eval ===")
    for off, s in decode_lite(img, STATUS_PROLOG, 0x14FB3C0):
        if any(k in s for k in ("LDRB", "CMP", "BEQ", "BNE", "CBZ", "CBNZ", "MOVS", "STRB", "BL ")):
            print(f"  0x{off:x}: {s}")


def analyze_set_app_direct_strb(img: memoryview) -> None:
    print("\n=== 12) Direct STRB to +0xBF4 (app_state) outside SET_APP? ===")
    hits = scan_strb_imm12(img, 0xBF4)
    print(f"  STRB.W +0xBF4 count={len(hits)}")
    for o, rt, rn in hits:
        # check if inside SET_APP function roughly 0x19916d2..0x1991800
        inside = 0x1991600 <= o <= 0x1991900
        win = decode_lite(img, max(0, o - 0x10), o + 8)
        movs = [s for _, s in win if s.startswith("MOVS")]
        mark = "IN_SET_APP" if inside else "OUTSIDE"
        print(f"  @0x{o:x} [{mark}] nearby_MOVS={movs}")


def main() -> None:
    path, img = load_img()
    digest = hashlib.sha256(bytes(img)).hexdigest()
    print(f"image={path}")
    print(f"size={len(img)} sha256={digest}")
    print(f"sha_prefix={digest[:8]} expect_live_B=449eeab3")

    scan_status_alt_entries(img)
    rows = analyze_set5(img)
    analyze_set5_gates(img, rows)
    analyze_bf6_stores(img)
    analyze_present_obj_writes(img)
    analyze_wrong_object(img)
    analyze_memcpy_into_present(img)
    analyze_alt_cmp(img)
    analyze_pin_skip_head(img)
    analyze_set_app_direct_strb(img)

    print("\n=== VERDICT ===")
    print("See anomalies above. Bypass exists only if SET#5 lacks Present==2")
    print("or a non-FN_A Present=2 writer is AP-elicitable under bans.")
    print("DONE")


if __name__ == "__main__":
    main()
