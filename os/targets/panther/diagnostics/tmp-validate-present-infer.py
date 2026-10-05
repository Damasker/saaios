#!/usr/bin/env python3
"""Validate present_infer model vs MAIN B GET_APP/+0xBF6/SET#5/STATUS_WRAP.

present_infer is NOT a dump of +0xBF6 (absent on 0x0200 wire). It maps
published app_state (+0xBF4 via GET_APP → byte 17) back through STATUS's
Present→SET_APP table. This script re-checks that table and whether PIN
sticky can hide a Present=2 that never made READY.
"""
from __future__ import annotations

import struct
from collections import Counter
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/"
    "os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

SET_APP = 0x19916D2
STATUS_WRAP = 0x14C6626
GET_APP = 0x18EC8C0
FN_A_STRB = 0x14F6A16
READY = 0x14FB5C6
SCAN_HI = min(len(IMG) - 4, 0x5A00000)


def u16(o: int) -> int:
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(pc: int, h1: int, h2: int) -> int | None:
    if (h1 & 0xF800) != 0xF000 or (h2 & 0xD000) != 0xD000:
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


def dump_window(start: int, end: int, label: str) -> None:
    print(f"\n=== {label} 0x{start:x}..0x{end:x} ===")
    o = start
    while o < end:
        h = u16(o)
        if (h & 0xE000) == 0xE000 and (h & 0x1800) != 0:
            h2 = u16(o + 2)
            extra = ""
            if (h & 0xFFF0) == 0xF890:
                extra = f" LDRB.W r{(h2 >> 12) & 0xF},[r{h & 0xF},#0x{h2 & 0xFFF:x}]"
            elif (h & 0xFFF0) == 0xF880:
                extra = f" STRB.W r{(h2 >> 12) & 0xF},[r{h & 0xF},#0x{h2 & 0xFFF:x}]"
            bt = bl_target(o, h, h2)
            if bt is not None:
                extra = f" BL 0x{bt:x}"
            print(f"  0x{o:x}: {h:04x} {h2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (h & 0xFF00) == 0x2800:
            extra = f" CMP r{(h >> 8) & 7},#{h & 0xFF}"
        elif (h & 0xFF00) == 0x2000:
            extra = f" MOVS r{(h >> 8) & 7},#{h & 0xFF}"
        elif (h & 0xF000) == 0xD000:
            cond = (h >> 8) & 0xF
            imm = h & 0xFF
            if imm >= 0x80:
                imm -= 0x100
            names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
            if cond < 14:
                extra = f" B{names[cond]}->0x{o + 4 + imm * 2:x}"
        print(f"  0x{o:x}: {h:04x}{extra}")
        o += 2


def main() -> None:
    dump_window(GET_APP & ~1, (GET_APP & ~1) + 0x40, "GET_APP body")

    print("\n=== BL→SET_APP with preceding MOVS r0,#imm ===")
    hits: list[tuple[int, int]] = []
    o = 0x100000
    while o < SCAN_HI:
        h, h2 = u16(o), u16(o + 2)
        bt = bl_target(o, h, h2)
        if bt is not None and (bt | 1) == (SET_APP | 1):
            imm = -1
            for back in range(2, 16, 2):
                hh = u16(o - back)
                if (hh & 0xFF00) == 0x2000:  # MOVS r0,#imm8
                    imm = hh & 0xFF
                    break
            hits.append((o, imm))
        o += 2
    print("count by imm:", dict(Counter(i for _, i in hits)))
    for site, imm in hits:
        mark = " << READY" if imm == 5 else ""
        print(f"  BL@0x{site:x} imm={imm}{mark}")

    dump_window(0x14FB5B0, 0x14FB5D8, "READY gate cluster")
    dump_window(0x14FB322, 0x14FB390, "STATUS head (GET skip?)")

    print("\n=== STATUS_WRAP callers ===")
    wrap = []
    o = 0x100000
    while o < SCAN_HI:
        h, h2 = u16(o), u16(o + 2)
        bt = bl_target(o, h, h2)
        if bt is not None and (bt | 1) == (STATUS_WRAP | 1):
            wrap.append(o)
        o += 2
    print("count:", len(wrap), [hex(x) for x in wrap])

    print("\n=== LDRB/STRB +0xBF6 inventory ===")
    ldrb, strb = [], []
    o = 0x100000
    while o < SCAN_HI:
        h, h2 = u16(o), u16(o + 2)
        if (h & 0xFFF0) == 0xF890 and (h2 & 0xFFF) == 0xBF6:
            ldrb.append(o)
        if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) == 0xBF6:
            strb.append(o)
        o += 2
    print("LDRB +0xBF6:", [hex(x) for x in ldrb])
    print("STRB +0xBF6:", [hex(x) for x in strb])

    print("\n=== FN_A Present=2 site ===")
    dump_window(FN_A_STRB - 0x10, FN_A_STRB + 0x10, "FN_A STRB=2")

    print("\n=== Verdict helpers ===")
    ready_hits = [s for s, i in hits if i == 5]
    print(f"SET#5 sites: {[hex(x) for x in ready_hits]} (expect sole near 0x{READY:x})")
    print(f"STRB +0xBF6 sole?: {strb == [0x14FB380]}")
    print(f"STATUS_WRAP only 2 callers?: {len(wrap) == 2}")
    print(
        "present_infer mapping (STATUS Present→app):\n"
        "  Present 0 → SET#2 PIN → infer notin_1_2_3\n"
        "  Present 1 → SET#3 PUK → was_1\n"
        "  Present 2 → SET#5 READY → was_2\n"
        "  Present 3 → SET#4 PERSO → was_3\n"
        "0x0200 byte17 = +0xBF4 (GET_APP), NOT +0xBF6.\n"
        "Caveat: while GET_APP==PIN, STATUS skips Present decision path;\n"
        "infer 'notin' means last published decision was PIN, not a live +0xBF6 peek.\n"
        "On EU, Present=2 writer is FN_A-only (never) so sticky-PIN hiding Present=2\n"
        "is not reachable without banned levers."
    )
    print("DONE")


if __name__ == "__main__":
    main()
