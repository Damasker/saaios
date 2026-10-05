#!/usr/bin/env python3
"""Harden: sit-base/sit-stream SIT_* name table + non-FN_A Present=2 writers."""
from __future__ import annotations

import re
import struct
from pathlib import Path

DIAG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
STREAM = (DIAG / "sit-stream.so").read_bytes()
BASE = (DIAG / "sit-base.so").read_bytes()
LIBSIT = (DIAG / "libsitril.so").read_bytes()
MAIN = (DIAG / "fw" / "saaios-probe-b-modem.bin").read_bytes()

FN_A_STORE = 0x14F6A16  # known Present=2 STRB in FN_A


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def bl_target(img, off):
    if off + 4 > len(img) or (off & 1):
        return None
    hi, lo = u16(img, off), u16(img, off + 2)
    if (hi & 0xF800) != 0xF000 or (lo & 0xD000) != 0xD000:
        return None
    s = (hi >> 10) & 1
    imm10 = hi & 0x3FF
    j1 = (lo >> 13) & 1
    j2 = (lo >> 11) & 1
    imm11 = lo & 0x7FF
    i1 = 1 - (j1 ^ s)
    i2 = 1 - (j2 ^ s)
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= (1 << 31):
            imm32 -= 1 << 32
    return (off + 4 + imm32) & 0xFFFFFFFF


print("=== All SIT_* ASCII names in sit-base / sit-stream / libsitril (SIM/APP/READY filter) ===")
for label, data in (("sit-base", BASE), ("sit-stream", STREAM), ("libsitril", LIBSIT)):
    names = sorted(set(m.group().decode() for m in re.finditer(rb"SIT_[A-Z0-9_]{3,80}", data)))
    print(f"\n-- {label} total SIT_*={len(names)} --")
    for n in names:
        if any(k in n for k in ("SIM", "APP", "READY", "PIN", "UICC", "CARD", "STATUS", "FORCE")):
            print(f"  {n}")

print("\n=== aarch64 MOVZ/MOVK #0x02xx in sit-stream (SIM family ids) ===")
# AArch64 MOVZ Wd, #imm16, LSL #0: 0x5280_0000 | (imm16<<5) | Rd
ids = set()
for o in range(0, len(STREAM) - 4, 4):
    w = struct.unpack_from("<I", STREAM, o)[0]
    if (w & 0xFF800000) == 0x52800000:  # MOVZ W
        imm16 = (w >> 5) & 0xFFFF
        if 0x0200 <= imm16 <= 0x02FF:
            ids.add(imm16)
    if (w & 0xFF800000) == 0xD2800000:  # MOVZ X
        imm16 = (w >> 5) & 0xFFFF
        if 0x0200 <= imm16 <= 0x02FF:
            ids.add(imm16)
print(" ", " ".join(f"{x:#06x}" for x in sorted(ids)))

print("\n=== MAIN: MOVS #2 then nearby STRB (Present=2 candidates, excl FN_A store) ===")
# Thumb MOVS Rd,#2 = 0x2002 | (Rd<<8) wait: encoding is 00100 Rd imm8 => 0x20xx with imm=2 => 0x2002,0x2102,...
cands = []
for o in range(0, len(MAIN) - 8, 2):
    w = u16(MAIN, o)
    if (w & 0xFF00) != 0x2000 or (w & 0xFF) != 2:
        continue
    # look forward 0..24B for STRB Rt,[Rn,#imm] narrow or wide
    for f in range(o + 2, min(len(MAIN) - 4, o + 0x30), 2):
        ww = u16(MAIN, f)
        # narrow STRB Rt,[Rn,#imm5]: 01110 imm5 Rn Rt
        if (ww & 0xF800) == 0x7000:
            imm5 = (ww >> 6) & 0x1F
            if imm5 == 0:  # PresentObj[0]
                cands.append((o, f, "strb_n#0", imm5))
        hi, lo = ww, u16(MAIN, f + 2) if f + 2 < len(MAIN) else 0
        if (hi & 0xFFF0) == 0xF880:
            imm12 = lo & 0xFFF
            if imm12 in (0, 0xBF6):
                cands.append((o, f, f"strb_w#{imm12:#x}", imm12))
print(f"candidates={len(cands)}")
for mov, st, kind, imm in cands[:40]:
    mark = " **FN_A**" if st == FN_A_STORE or abs(st - FN_A_STORE) < 8 else ""
    print(f"  MOVS#2@{mov:#x} -> {kind}@{st:#x}{mark}")

# Also: who BLs FN_A (already known) vs other Present stores via getobj #0x636c
print("\n=== getobj(#0x636c) sites (PresentObj) — prior claim 7 ===")
# MOVW #0x636c then likely BL getobj — search imm 0x636c
hits = []
for o in range(0, len(MAIN) - 4, 2):
    hi, lo = u16(MAIN, o), u16(MAIN, o + 2)
    if (hi & 0xFBF0) == 0xF240:
        i = (hi >> 10) & 1
        imm4 = hi & 0xF
        imm3 = (lo >> 12) & 7
        imm8 = lo & 0xFF
        imm16 = (imm4 << 12) | (i << 11) | (imm3 << 8) | imm8
        if imm16 == 0x636C:
            hits.append(o)
print("MOVW #0x636c count", len(hits), [hex(x) for x in hits])

print("\n=== DONE ===")
