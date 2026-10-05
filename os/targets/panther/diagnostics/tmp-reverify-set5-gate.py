#!/usr/bin/env python3
"""Fresh-eyes re-verify SET_APP#5 / Present(+0xBF6) gate on MAIN B.

Stock image: saaios-probe-b-modem.bin (sha256 449eeab3…).
Goal: confirm whether READY(#5) is really gated by Present==2 only,
or whether other Present values / fields / STATUS entry points can
reach SET#5.
"""
from __future__ import annotations

import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/"
    "os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
SET_APP = 0x19916D2  # Thumb entry (odd)
STATUS_PROLOG = 0x14FB322
READY_SITE = 0x14FB5C6
BF6_STRB = 0x14FB380


def u16(img: memoryview, off: int) -> int:
    return struct.unpack_from("<H", img, off)[0]


def u32(img: memoryview, off: int) -> int:
    return struct.unpack_from("<I", img, off)[0]


def bl_target(pc: int, h1: int, h2: int) -> int | None:
    """Decode Thumb BL; pc = address of first halfword. Return even target."""
    if (h1 & 0xF800) != 0xF000:
        return None
    if (h2 & 0xD000) != 0xD000:  # BL (not BLX)
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


def decode_thumb_window(img: memoryview, start: int, end: int) -> list[tuple[int, str]]:
    """Lightweight Thumb decoder for gate-relevant ops only."""
    out: list[tuple[int, str]] = []
    o = start
    while o < end:
        h = u16(img, o)
        # 32-bit Thumb encodings start with 0b11101 / 0b11110 / 0b11111
        if (h & 0xE000) == 0xE000 and (h & 0x1800) != 0:
            if o + 2 >= end:
                out.append((o, f"trunc {h:04x}"))
                break
            h2 = u16(img, o + 2)
            # BL
            tgt = bl_target(o, h, h2)
            if tgt is not None:
                tag = "SET_APP" if (tgt | 1) == (SET_APP | 1) else ""
                out.append((o, f"BL 0x{tgt:x} {tag}".rstrip()))
                o += 4
                continue
            # LDRB.W Rt,[Rn,#imm12]  encoding T3: F89n tttt iiii iiii iiii
            if (h & 0xFFF0) == 0xF890:
                rt = (h2 >> 12) & 0xF
                rn = h & 0xF
                imm12 = h2 & 0xFFF
                out.append((o, f"LDRB.W r{rt},[r{rn},#0x{imm12:x}]"))
                o += 4
                continue
            # STRB.W Rt,[Rn,#imm12]  F88n
            if (h & 0xFFF0) == 0xF880:
                rt = (h2 >> 12) & 0xF
                rn = h & 0xF
                imm12 = h2 & 0xFFF
                out.append((o, f"STRB.W r{rt},[r{rn},#0x{imm12:x}]"))
                o += 4
                continue
            # LDRH.W / etc — keep raw
            # B.W
            if (h & 0xF800) == 0xF000 and (h2 & 0xD000) == 0x9000:
                # unconditional/conditional B.W — skip detailed
                out.append((o, f"B.W/raw {h:04x}{h2:04x}"))
                o += 4
                continue
            out.append((o, f"t32 {h:04x} {h2:04x}"))
            o += 4
            continue

        # 16-bit
        # MOVS Rd,#imm8
        if (h & 0xF800) == 0x2000:
            rd = (h >> 8) & 7
            imm = h & 0xFF
            out.append((o, f"MOVS r{rd},#{imm}"))
            o += 2
            continue
        # CMP Rn,#imm8
        if (h & 0xF800) == 0x2800:
            rn = (h >> 8) & 7
            imm = h & 0xFF
            out.append((o, f"CMP r{rn},#{imm}"))
            o += 2
            continue
        # CMP Rn,Rm (lo)
        if (h & 0xFFC0) == 0x4280:
            out.append((o, f"CMP r{h & 7},r{(h >> 3) & 7}"))
            o += 2
            continue
        # CBZ / CBNZ
        if (h & 0xF500) == 0xB100:
            rn = h & 7
            imm5 = (h >> 3) & 0x1F
            i = (h >> 9) & 1
            op = "CBNZ" if (h & 0x0800) else "CBZ"
            dest = o + 4 + (i << 6) + (imm5 << 1)
            out.append((o, f"{op} r{rn},0x{dest:x}"))
            o += 2
            continue
        # BEQ / BNE / etc conditional branch
        if (h & 0xF000) == 0xD000 and (h & 0x0F00) != 0x0E00 and (h & 0x0F00) != 0x0F00:
            conds = [
                "EQ",
                "NE",
                "CS",
                "CC",
                "MI",
                "PL",
                "VS",
                "VC",
                "HI",
                "LS",
                "GE",
                "LT",
                "GT",
                "LE",
            ]
            cond = conds[(h >> 8) & 0xF]
            imm8 = h & 0xFF
            if imm8 >= 0x80:
                imm8 -= 0x100
            dest = o + 4 + (imm8 << 1)
            out.append((o, f"B{cond} 0x{dest:x}"))
            o += 2
            continue
        # LDRB Rt,[Rn,#imm5]
        if (h & 0xF800) == 0x7800:
            rt = h & 7
            rn = (h >> 3) & 7
            imm5 = (h >> 6) & 0x1F
            out.append((o, f"LDRB r{rt},[r{rn},#{imm5}]"))
            o += 2
            continue
        # STRB Rt,[Rn,#imm5]
        if (h & 0xF800) == 0x7000:
            rt = h & 7
            rn = (h >> 3) & 7
            imm5 = (h >> 6) & 0x1F
            out.append((o, f"STRB r{rt},[r{rn},#{imm5}]"))
            o += 2
            continue
        # BX lr
        if h == 0x4770:
            out.append((o, "BX lr"))
            o += 2
            continue
        # PUSH / POP rough
        if (h & 0xFE00) == 0xB400:
            out.append((o, f"PUSH/POP {h:04x}"))
            o += 2
            continue
        out.append((o, f"h16 {h:04x}"))
        o += 2
    return out


def find_bl_to_set_app(img: memoryview) -> list[tuple[int, int]]:
    """Return (site, preceding_movs_imm_or_-1) for every BL→SET_APP."""
    hits = []
    # Scan whole MAIN-ish range used previously
    start, end = 0x100000, 0x1C00000
    end = min(end, len(img) - 4)
    o = start
    while o < end:
        h1 = u16(img, o)
        if (h1 & 0xF800) == 0xF000:
            h2 = u16(img, o + 2)
            tgt = bl_target(o, h1, h2)
            if tgt is not None and (tgt | 1) == (SET_APP | 1):
                # look back up to 12 bytes for MOVS r0,#imm
                imm = -1
                for back in range(2, 14, 2):
                    hh = u16(img, o - back)
                    if (hh & 0xF800) == 0x2000 and ((hh >> 8) & 7) == 0:
                        imm = hh & 0xFF
                        break
                hits.append((o, imm))
                o += 4
                continue
        o += 2
    return hits


def scan_ldrb_bf6(img: memoryview) -> list[int]:
    """All LDRB.W Rt,[Rn,#0xBF6] sites."""
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF890:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == 0xBF6:
            hits.append(o)
    return hits


def scan_strb_bf6(img: memoryview) -> list[int]:
    hits = []
    end = min(len(img) - 4, 0x1C00000)
    for o in range(0x100000, end, 2):
        h = u16(img, o)
        if (h & 0xFFF0) != 0xF880:
            continue
        h2 = u16(img, o + 2)
        if (h2 & 0xFFF) == 0xBF6:
            hits.append(o)
    return hits


def nearby_cmp_after(dis: list[tuple[int, str]], after_off: int, limit: int = 12) -> list[str]:
    """CMP / branch lines after a given offset."""
    got = []
    started = False
    for off, s in dis:
        if off >= after_off:
            started = True
        if not started:
            continue
        if s.startswith(("CMP", "BEQ", "BNE", "BCS", "BCC", "CBZ", "CBNZ", "MOVS", "BL ")):
            got.append(f"  0x{off:x}: {s}")
        if len(got) >= limit:
            break
    return got


def main() -> None:
    raw = PATH.read_bytes()
    img = memoryview(raw)
    print(f"image={PATH.name} size={len(raw)} sha256_prefix=449eeab3 (expect)")

    print("\n=== A) All BL → SET_APP (0x19916d2) with preceding MOVS r0,#imm ===")
    hits = find_bl_to_set_app(img)
    by_imm: dict[int, list[int]] = {}
    for site, imm in hits:
        by_imm.setdefault(imm, []).append(site)
        print(f"  BL@0x{site:x}  MOVS r0,#{imm}")
    print(f"total_BL={len(hits)}")
    for imm in sorted(by_imm):
        sites = ", ".join(f"0x{s:x}" for s in by_imm[imm])
        print(f"  imm#{imm}: {len(by_imm[imm])} → {sites}")

    print("\n=== B) LDRB.W +0xBF6 inventory ===")
    ldrb = scan_ldrb_bf6(img)
    for o in ldrb:
        print(f"  LDRB.W @0x{o:x}")
    print(f"count={len(ldrb)}")

    print("\n=== C) STRB.W +0xBF6 inventory ===")
    strb = scan_strb_bf6(img)
    for o in strb:
        print(f"  STRB.W @0x{o:x}")
    print(f"count={len(strb)}")

    print("\n=== D) STATUS window decode 0x14fb322 .. 0x14fb620 ===")
    dis = decode_thumb_window(img, STATUS_PROLOG, 0x14FB620)
    # Print only gate-relevant lines
    for off, s in dis:
        if any(
            k in s
            for k in (
                "LDRB",
                "STRB",
                "CMP",
                "CBZ",
                "CBNZ",
                "BEQ",
                "BNE",
                "MOVS",
                "BL ",
                "B.W",
            )
        ):
            mark = ""
            if off == READY_SITE or (off <= READY_SITE < off + 4 and "BL" in s):
                mark = "  << READY SET#5"
            if off == BF6_STRB or ("STRB.W" in s and "0xbf6" in s.lower()):
                mark = "  << sole +0xBF6 STRB"
            print(f"  0x{off:x}: {s}{mark}")

    print("\n=== E) READY site local dump (0x14fb5a0..0x14fb5e0) ===")
    for off, s in decode_thumb_window(img, 0x14FB5A0, 0x14FB5E0):
        print(f"  0x{off:x}: {s}")

    print("\n=== F) For each #5 BL: what CMP precedes within 0x40 bytes? ===")
    for site, imm in hits:
        if imm != 5:
            continue
        window = decode_thumb_window(img, site - 0x40, site + 4)
        print(f"-- BL#5 @0x{site:x} --")
        for off, s in window:
            if any(
                k in s
                for k in (
                    "LDRB",
                    "CMP",
                    "CBZ",
                    "CBNZ",
                    "BEQ",
                    "BNE",
                    "MOVS",
                    "BL ",
                )
            ):
                print(f"  0x{off:x}: {s}")

    print("\n=== G) After each LDRB.W +0xBF6: following CMP immediates ===")
    for o in ldrb:
        win = decode_thumb_window(img, o, o + 0x30)
        cmps = [s for _, s in win if s.startswith("CMP")]
        print(f"  LDRB@0x{o:x} → CMPs: {cmps}")

    print("\n=== H) Is there MOVS r0,#5 NOT preceded by CMP #2? ===")
    for site, imm in hits:
        if imm != 5:
            continue
        win = decode_thumb_window(img, site - 0x30, site)
        cmp_imms = []
        for off, s in win:
            if s.startswith("CMP r") and "#" in s:
                # CMP rN,#imm
                try:
                    cmp_imms.append(int(s.split("#", 1)[1]))
                except ValueError:
                    pass
        last = cmp_imms[-1] if cmp_imms else None
        verdict = "OK_Present_eq_2" if last == 2 else f"ANOMALY last_cmp={last} all={cmp_imms}"
        print(f"  #5@0x{site:x}: {verdict}")

    print("\n=== I) GET_APP CMP {1,4} at STATUS head (PIN skip?) ===")
    for off, s in decode_thumb_window(img, 0x14FB322, 0x14FB380):
        if any(k in s for k in ("CMP", "BEQ", "BNE", "BL ", "MOVS", "LDRB")):
            print(f"  0x{off:x}: {s}")

    print("\n=== J) Hex sanity READY cluster ===")
    chunk = bytes(img[0x14FB5B8:0x14FB5D0])
    print("  " + chunk.hex(" "))
    # Expected: LDRB.W ..,#0xBF6 ; CMP #2 ; Bxx ; MOVS r0,#5 ; BL SET_APP
    print("DONE")


if __name__ == "__main__":
    main()
