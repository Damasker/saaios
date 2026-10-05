#!/usr/bin/env python3
"""RE: PresentObj ctor default + who writes Present[0] / +0xBF6 as 0 vs 2.

Hypothesis under test: Present defaults to 2 at object init; our path clears to 0.
Prior claim: ctor STRB Present=0 @0x1a552a4.
"""
from __future__ import annotations

import struct
from collections import defaultdict
from pathlib import Path

CANDIDATES = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics\fw\saaios-probe-b-modem.bin"),
]
IMG_PATH = next(p for p in CANDIDATES if p.exists())
IMG = IMG_PATH.read_bytes()
print(f"IMG={IMG_PATH}")

SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)
GETOBJ = 0x20EA040
SIZE = 0x636C
CTOR_CLAIM = 0x1A552A4
FN_A_STRB = 0x14F6A16
STATUS_BF6 = 0x14FB380


def u16(o: int) -> int:
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o: int) -> int | None:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32


def movw_imm(o: int) -> int | None:
    """MOVW encoding: F240 / F2C0 family immediate."""
    h, h2 = u16(o), u16(o + 2)
    if (h & 0xFBF0) != 0xF240 or (h2 & 0x8000):
        return None
    i = (h >> 10) & 1
    imm = (i << 11) | ((h & 0xF) << 12) | (((h2 >> 12) & 7) << 8) | (h2 & 0xFF)
    return imm


def dump(start: int, end: int, label: str) -> None:
    print(f"\n=== {label} {hex(start)}..{hex(end)} ===")
    o = start
    while o < end:
        h = u16(o)
        if (h & 0xE000) == 0xE000 and (h & 0x1800) != 0:
            h2 = u16(o + 2)
            extra = ""
            bt = bl_target(o)
            if bt is not None:
                extra = f" BL->{hex(bt)}"
            elif (h & 0xFFF0) == 0xF880:
                extra = f" STRB.W r{(h2 >> 12) & 0xF},[r{h & 0xF},#{hex(h2 & 0xFFF)}]"
            elif (h & 0xFFF0) == 0xF890:
                extra = f" LDRB.W r{(h2 >> 12) & 0xF},[r{h & 0xF},#{hex(h2 & 0xFFF)}]"
            elif (h & 0xFFF0) == 0xF8C0:
                extra = f" STR.W r{(h2 >> 12) & 0xF},[r{h & 0xF},#{hex(h2 & 0xFFF)}]"
            print(f"  {hex(o)}: {h:04x} {h2:04x}{extra}")
            o += 4
            continue
        extra = ""
        if (h & 0xFF00) == 0x2000:
            extra = f" MOVS r{(h >> 8) & 7},#{h & 0xFF}"
        elif (h & 0xF800) == 0x7000:
            rt, rn, imm5 = h & 7, (h >> 3) & 7, (h >> 6) & 0x1F
            extra = f" STRB r{rt},[r{rn},#{imm5}]"
        elif (h & 0xF800) == 0x7800:
            rt, rn, imm5 = h & 7, (h >> 3) & 7, (h >> 6) & 0x1F
            extra = f" LDRB r{rt},[r{rn},#{imm5}]"
        print(f"  {hex(o)}: {h:04x}{extra}")
        o += 2


def lookback_imm_to_rt(store_o: int, rt: int, window: int = 32) -> int | None:
    for a in range(store_o - 2, max(SCAN_LO, store_o - window) - 1, -2):
        h = u16(a)
        if (h & 0xFF00) == 0x2000 and ((h >> 8) & 7) == (rt & 7):
            return h & 0xFF
        # MOVW rd,#imm
        if (h & 0xFBF0) == 0xF240:
            h2 = u16(a + 2)
            if not (h2 & 0x8000):
                rd = (h2 >> 8) & 0xF
                if rd == rt:
                    imm = movw_imm(a)
                    return imm
    return None


def nearby_636c(o: int, radius: int = 0x140) -> bool:
    for b in range(max(SCAN_LO, o - radius), min(SCAN_HI - 4, o + 0x40), 2):
        imm = movw_imm(b)
        if imm == SIZE:
            return True
    return False


def main() -> None:
    print(f"image_bytes={len(IMG)} sha_prefix_expect=449eeab3")

    # --- ctor claim ---
    dump(CTOR_CLAIM - 0x40, CTOR_CLAIM + 0x60, "claimed Present=0 ctor @0x1a552a4")
    print(f"\nwords @ctor: {u16(CTOR_CLAIM):04x} {u16(CTOR_CLAIM+2):04x}")

    # Find getobj(#0x636c) call sites
    print("\n=== BL -> getobj with nearby MOVW #0x636c ===")
    sites = []
    o = SCAN_LO
    while o < SCAN_HI:
        if bl_target(o) == GETOBJ:
            has = nearby_636c(o, 0x80)
            if has:
                sites.append(o)
                print(f"  BL getobj @{hex(o)}")
                dump(o - 0x30, o + 0x80, f"getobj#636c @{hex(o)}")
        o += 2
    print(f"getobj#636c sites: {len(sites)}")

    # After each getobj, look for early STRB [obj,#0] with imm 0/1/2/3
    print("\n=== Early STRB [*,#0] within +0x100 of getobj#636c BL (init candidates) ===")
    for bl in sites:
        for a in range(bl + 4, min(bl + 0x100, SCAN_HI - 2), 2):
            h = u16(a)
            # STRB rt,[rn,#0]
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) == 0:
                rt = h & 7
                imm = lookback_imm_to_rt(a, rt)
                print(f"  after {hex(bl)}: STRB @{hex(a)} lookback_imm={imm}")
            if (h & 0xFFF0) == 0xF880:
                h2 = u16(a + 2)
                if (h2 & 0xFFF) == 0:
                    rt = (h2 >> 12) & 0xF
                    imm = lookback_imm_to_rt(a, rt)
                    print(f"  after {hex(bl)}: STRB.W @{hex(a)} #0 lookback_imm={imm}")

    # All STRB.W +0xBF6
    print("\n=== ALL STRB.W #+0xBF6 ===")
    strb_bf6 = []
    o = SCAN_LO
    while o < SCAN_HI - 4:
        h, h2 = u16(o), u16(o + 2)
        if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) == 0xBF6:
            rt = (h2 >> 12) & 0xF
            imm = lookback_imm_to_rt(o, rt, 48)
            strb_bf6.append((o, rt, imm))
            print(f"  {hex(o)} STRB.W r{rt},[r{h & 0xF},#0xBF6] lookback_imm={imm}")
        o += 2
    print(f"count={len(strb_bf6)}")

    # PresentObj[0]=N: MOVS #N + STRB #0 near #636c
    print("\n=== PresentObj[0] STRB near #636c (MOVS imm + STRB #0) ===")
    by_imm: dict[int, list[tuple[int, int]]] = defaultdict(list)
    o = SCAN_LO
    while o < SCAN_HI - 4:
        h = u16(o)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) in (0, 1, 2, 3):
            rd = (h >> 8) & 7
            immv = h & 0xFF
            for a in range(o + 2, min(o + 12, SCAN_HI - 2), 2):
                h2 = u16(a)
                hit = False
                if (h2 & 0xF800) == 0x7000 and (h2 & 7) == rd and ((h2 >> 6) & 0x1F) == 0:
                    hit = True
                if (h2 & 0xFFF0) == 0xF880:
                    h3 = u16(a + 2)
                    if ((h3 >> 12) & 0xF) == rd and (h3 & 0xFFF) == 0:
                        hit = True
                if hit and nearby_636c(o, 0x120):
                    by_imm[immv].append((o, a))
                    break
        o += 2
    for immv in sorted(by_imm):
        print(f"  Present={immv}: {len(by_imm[immv])} sites")
        for mov, st in by_imm[immv]:
            print(f"    MOVS@{hex(mov)} STRB@{hex(st)}")

    # Who writes Present=0 specifically (clear candidates)
    print("\n=== Present=0 writers near #636c (clear candidates) ===")
    for mov, st in by_imm.get(0, []):
        dump(st - 0x30, st + 0x20, f"Present=0 @{hex(st)}")

    # Confirm FN_A Present=2
    dump(FN_A_STRB - 0x20, FN_A_STRB + 0x10, "FN_A Present=2")

    # STATUS BF6 copy source
    dump(STATUS_BF6 - 0x30, STATUS_BF6 + 0x10, "STATUS +0xBF6 copy")

    # Search memset / bulk zero of PresentObj size
    print("\n=== MOVW #0x636c near BL (possible memset/init of whole object) ===")
    o = SCAN_LO
    n = 0
    while o < SCAN_HI - 4 and n < 40:
        imm = movw_imm(o)
        if imm == SIZE:
            # look for BL within ±0x20
            for a in range(max(SCAN_LO, o - 0x20), min(SCAN_HI, o + 0x28), 2):
                bt = bl_target(a)
                if bt is not None:
                    print(f"  MOVW#636c @{hex(o)} BL@{hex(a)}->{hex(bt)}")
                    n += 1
                    break
        o += 2

    print("\n=== VERDICT SUMMARY ===")
    print(
        "Init default Present=2? "
        "See early STRB after getobj and Present=0/2 site lists above."
    )
    print(f"STRB +0xBF6 count={len(strb_bf6)} (expect sole STATUS mirror)")
    print(f"Present=2 near #636c: {[hex(s) for _, s in by_imm.get(2, [])]}")
    print(f"Present=0 near #636c: {[hex(s) for _, s in by_imm.get(0, [])]}")
    print("DONE")


if __name__ == "__main__":
    main()
