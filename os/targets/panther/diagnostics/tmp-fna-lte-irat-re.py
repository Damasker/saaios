#!/usr/bin/env python3
"""Fast RO: FN_A Present=2 — CDMA-only vs LTE/SADR path (bounded scans)."""
from __future__ import annotations
import hashlib
import struct
from pathlib import Path

PATH = Path("/tmp/modem-re/b.bin")
FN_A = 0x14F692C
WRAP_A = 0x14C380E
WRAP_SIM = 0x14F6D02
LATCH = 0x14C388E
HELPER = 0x14F6900
STATUS_WRAP = 0x14C6626
STATUS = 0x14FB322
SADR_CALLER = 0x14B7CA8  # prior
L1TUNNEL = 0x1A3E6CC


def bl_target(raw: bytes, off: int) -> int | None:
    if off + 4 > len(raw):
        return None
    w0, w1 = struct.unpack_from("<HH", raw, off)
    if (w0 & 0xF800) != 0xF000 or (w1 & 0xD000) != 0xD000:
        return None
    if (w1 & 0x1000) == 0:
        return None
    s = (w0 >> 10) & 1
    imm10 = w0 & 0x3FF
    j1 = (w1 >> 13) & 1
    j2 = (w1 >> 11) & 1
    imm11 = w1 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= -1 << 25
    return ((off + 4 + imm32) & 0xFFFFFFFF) & ~1


def find_bl_in(raw: bytes, target: int, lo: int, hi: int) -> list[int]:
    tgt = target & ~1
    hits = []
    hi = min(hi, len(raw) - 3)
    o = lo & ~1
    while o < hi:
        t = bl_target(raw, o)
        if t is not None and (t & ~1) == tgt:
            hits.append(o)
        o += 2
    return hits


def find_all_bl(raw: bytes, target: int) -> list[int]:
    # Whole image but vectorized-ish: only check candidate halfwords F0xx
    tgt = target & ~1
    hits = []
    # scan every 2 bytes — still OK on native /tmp (~few sec for 98MB)
    data = memoryview(raw)
    for o in range(0, len(raw) - 3, 2):
        if data[o + 1] & 0xF8 != 0xF0:
            continue
        t = bl_target(raw, o)
        if t is not None and (t & ~1) == tgt:
            hits.append(o)
    return hits


def movw_movt(raw: bytes, off: int) -> int | None:
    if off + 4 > len(raw) or off < 4:
        return None
    w0, w1 = struct.unpack_from("<HH", raw, off)
    if (w0 & 0xFBF0) != 0xF2C0 or (w1 & 0x8000) != 0:
        return None
    i = (w0 >> 10) & 1
    imm4 = w0 & 0xF
    imm3 = (w1 >> 12) & 7
    rd = (w1 >> 8) & 0xF
    imm8 = w1 & 0xFF
    imm_hi = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
    for back in range(4, 64, 2):
        p = off - back
        if p < 0:
            break
        a0, a1 = struct.unpack_from("<HH", raw, p)
        if (a0 & 0xFBF0) != 0xF240 or (a1 & 0x8000) != 0:
            continue
        i2 = (a0 >> 10) & 1
        rd2 = (a1 >> 8) & 0xF
        if rd2 != rd:
            continue
        imm_lo = ((a0 & 0xF) << 12) | (i2 << 11) | (((a1 >> 12) & 7) << 8) | (a1 & 0xFF)
        return (imm_hi << 16) | imm_lo
    return None


def cstr(raw: bytes, off: int, n: int = 90) -> bytes | None:
    if off < 0 or off >= len(raw) or not (0x20 <= raw[off] < 0x7F):
        return None
    end = raw.find(b"\0", off, off + n)
    if end < 0:
        end = off + n
    return raw[off:end]


def name_at_imm(raw: bytes, imm: int) -> bytes | None:
    for cand in (imm, imm - 0x40000000 if imm >= 0x40000000 else None):
        if cand is None:
            continue
        s = cstr(raw, cand)
        if s:
            return s
    return None


def dump(raw: bytes, lo: int, hi: int) -> None:
    o = lo & ~1
    while o < hi and o + 2 <= len(raw):
        h = struct.unpack_from("<H", raw, o)[0]
        if (h & 0xF800) in (0xF000, 0xF800, 0xE800) and o + 4 <= len(raw):
            w1 = struct.unpack_from("<H", raw, o + 2)[0]
            bt = bl_target(raw, o)
            print(f"  {o:#x}: {h:04x} {w1:04x}" + (f" -> {bt:#x}" if bt else ""))
            o += 4
        else:
            print(f"  {o:#x}: {h:04x}")
            o += 2


def main() -> None:
    raw = PATH.read_bytes()
    h = hashlib.sha256(raw).hexdigest()
    print(f"size={len(raw)} sha256={h}")
    assert h.startswith("449eeab3")

    print("\n=== strings (presence) ===")
    for s in [
        b"MMC_LTEL1_CDMA_MEAS_RESULT_IND",
        b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF",
        b"MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP",
        b"No CDMA in SupportedRatMap",
        b"MMC_LTEL1_UMTS_MEASURE_CNF",
        b"MMC_LTEL1_NR_MEASURE",
        b"LTE_MEAS_RESULT",
        b"NR_MEAS_RESULT",
    ]:
        print(f"  {s.decode():40s} n={raw.count(s)}")

    print("\n=== ALL BL -> FN_A (whole MAIN) ===")
    bls = find_all_bl(raw, FN_A)
    print(f"count={len(bls)} sites={[hex(x) for x in bls]}")
    for o in bls:
        # look back for 0x4106 case name
        found = []
        for b in range(max(0, o - 0x100), o, 2):
            imm = movw_movt(raw, b)
            if imm and (imm >> 16) == 0x4106:
                found.append((hex(b), hex(imm), name_at_imm(raw, imm)))
        print(f"  BL@{hex(o)} nearby_4106={found[-3:]}")

    print("\n=== BL counts for wraps / STATUS ===")
    for name, tgt in [
        ("WRAP_A", WRAP_A),
        ("WRAP_SIM", WRAP_SIM),
        ("LATCH", LATCH),
        ("HELPER", HELPER),
        ("STATUS_WRAP", STATUS_WRAP),
        ("STATUS", STATUS),
    ]:
        hits = find_all_bl(raw, tgt)
        print(f"  {name:12s} n={len(hits)} {[hex(x) for x in hits]}")

    print("\n=== FN_A: CMP#3 / STRB Present window ===")
    dump(raw, FN_A, FN_A + 0x100)

    print("\n=== MMC dispatcher MEASURE/LATCH/SADR cases (0x14b7700..0x14b7e00) ===")
    for o in range(0x14B7700, 0x14B7E00, 2):
        imm = movw_movt(raw, o)
        if not imm or (imm >> 16) != 0x4106:
            continue
        s = name_at_imm(raw, imm)
        if not s:
            continue
        if not any(k in s for k in (b"CDMA", b"SADR", b"MEASURE", b"LATCH", b"UMTS", b"NR", b"LTE")):
            continue
        bls_n = []
        for b in range(o, min(len(raw) - 4, o + 0x60), 2):
            t = bl_target(raw, b)
            if t is None:
                continue
            lab = {
                WRAP_A: "WRAP_A",
                WRAP_SIM: "WRAP_SIM",
                LATCH: "LATCH",
                FN_A: "FN_A",
                STATUS_WRAP: "STATUS_WRAP",
                HELPER: "HELPER",
                STATUS: "STATUS",
            }.get(t & ~1, hex(t & ~1))
            bls_n.append((hex(b), lab))
        print(f"  @{hex(o)} {s!r}")
        print(f"     next_bls={bls_n[:6]}")

    print("\n=== SADR_MEASURE_RSP caller region vs FN_A ===")
    # Confirm STATUS_WRAP callers and whether FN_A is in same functions
    sw = find_all_bl(raw, STATUS_WRAP)
    for o in sw:
        near_fna = [hex(x) for x in bls if abs(x - o) < 0x800]
        # case name near
        names = []
        for b in range(max(0, o - 0x80), o + 0x20, 2):
            imm = movw_movt(raw, b)
            if imm and (imm >> 16) == 0x4106:
                names.append(name_at_imm(raw, imm))
        print(f"  STATUS_WRAP@{hex(o)} names={names} near_FN_A={near_fna}")

    print("\n=== Does any LTE/UMTS/NR MEASURE case BL WRAP_A/FN_A? ===")
    # Scan broader MMC switch 0x14b7000..0x14b9000 for MEASURE cases
    lte_hits = []
    for o in range(0x14B7000, 0x14B9000, 2):
        imm = movw_movt(raw, o)
        if not imm or (imm >> 16) != 0x4106:
            continue
        s = name_at_imm(raw, imm)
        if not s or b"MEASURE" not in s and b"LATCH" not in s and b"SADR" not in s:
            continue
        # find BL in following 0x80
        targets = []
        for b in range(o, min(len(raw) - 4, o + 0x80), 2):
            t = bl_target(raw, b)
            if t is None:
                continue
            tt = t & ~1
            if tt in (WRAP_A, WRAP_SIM, LATCH, FN_A, STATUS_WRAP, HELPER):
                targets.append({
                    WRAP_A: "WRAP_A",
                    WRAP_SIM: "WRAP_SIM",
                    LATCH: "LATCH",
                    FN_A: "FN_A",
                    STATUS_WRAP: "STATUS_WRAP",
                    HELPER: "HELPER",
                }[tt])
        if targets or (s and (b"CDMA" in s or b"SADR" in s)):
            lte_hits.append((hex(o), s.decode(errors="replace"), targets))
    for row in lte_hits:
        print(f"  {row}")

    # arg==3 exclusivity: FN_A CMP #3
    print("\n=== FN_A arg gate (search CMP #3 in FN_A+0x200) ===")
    for o in range(FN_A, FN_A + 0x200, 2):
        h = struct.unpack_from("<H", raw, o)[0]
        # CMP Rn, #imm8 : 00101 nnn iiiiiiii
        if (h & 0xF800) == 0x2800 and (h & 0xFF) == 3:
            print(f"  CMP #3 @{hex(o)} insn={h:04x}")
        # MOVS Rd,#2 then STRB
        if (h & 0xF800) == 0x2000 and (h & 0xFF) == 2:
            print(f"  MOVS #2 @{hex(o)}")

    print("\n=== VERDICT ===")
    print(f"BL->FN_A count={len(bls)} sites={[hex(x) for x in bls]}")
    cdma_only = len(bls) == 2
    print(f"CDMA-only latch model: {'CONFIRMED' if cdma_only else 'RECHECK'}")
    print("SADR_MEASURE_RSP -> STATUS_WRAP (READY eval), not Present=2 latch.")
    print("LTE/NR cannot feed FN_A arg==3 on this MAIN unless BL count>2 or alias.")


if __name__ == "__main__":
    main()
