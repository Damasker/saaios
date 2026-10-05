#!/usr/bin/env python3
"""Map SIT 0x0704 → START_NETWORK → RCM error 2. Enumerate every CMP gate."""
from __future__ import annotations
import struct
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/"
    "diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

GET_APP = 0x18EC8C0
START_NET = 0x18E8028
SIT_REG = 0x20D1AFA


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def bl_target(o):
    if o + 4 > len(IMG):
        return None
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


def bcond_w(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0x8000:
        return None
    s = (hw >> 10) & 1
    cond = (hw >> 6) & 0xF
    imm6 = hw & 0x3F
    j1 = (hw2 >> 13) & 1
    j2 = (hw2 >> 11) & 1
    imm11 = hw2 & 0x7FF
    i1 = ~(j1 ^ s) & 1
    i2 = ~(j2 ^ s) & 1
    imm32 = (s << 20) | (i1 << 19) | (i2 << 18) | (imm6 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 21) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32, cond


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def dump(start, end, lab):
    print(f"\n=== {lab} [{hex(start)}..{hex(end)}] ===")
    o = start
    while o < end:
        hw = u16(o)
        extra = ""
        mw, b = movw(o), bl_target(o)
        bc = bcond_w(o)
        if mw:
            extra = f" MOVW r{mw[1]},#{hex(mw[0])}"
        if b is not None:
            tag = ""
            if b == GET_APP:
                tag = " GET_APP"
            elif abs(b - START_NET) < 8:
                tag = " START_NET"
            extra = f" BL->{hex(b)}{tag}"
        if bc:
            extra += f" Bcond.W->{hex(bc[0])} c={bc[1]}"
        if (hw & 0xFF00) == 0x2000:
            extra += f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFF00) == 0x2800:
            extra += f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if (hw & 0xFFF0) == 0xF890:
            hw2 = u16(o + 2)
            extra += f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            print(f" {hex(o)}:{hw:04x} {u16(o+2):04x}{extra}")
            o += 4
        else:
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# Error sites with STRB #0xa near MOVS#2 — dump those that also see 0x704 / GET_APP / START_NET
err_sites = [
    0x1929274, 0x192BA44, 0x192C7C4, 0x192CFF8, 0x192D0B0, 0x192DF78,
    0x192E074, 0x192E1D6, 0x1940BB2, 0x194215C, 0x19481CE, 0x1965FC4,
    0x1735B68, 0x1735C04,
]
print("=== error_raw STRB candidates with context ===")
for s in err_sites:
    ctx = []
    for j in range(s - 0x80, s + 0x80, 2):
        mw = movw(j)
        if mw and mw[0] in (0x704, 0x703, 0x701, 0x70A, 0x710, 2):
            ctx.append(f"{hex(j)}:#{hex(mw[0])}")
        b = bl_target(j)
        if b == GET_APP:
            ctx.append(f"{hex(j)}:GET_APP")
        if b == START_NET:
            ctx.append(f"{hex(j)}:START_NET")
        # LDRB +0xBF4/+0xBF5/+0xBF6
        if (u16(j) & 0xFFF0) == 0xF890:
            hw2 = u16(j + 2)
            imm = hw2 & 0xFFF
            if imm in (0xBF4, 0xBF5, 0xBF6, 0xC3):
                ctx.append(f"{hex(j)}:LDRB+{hex(imm)}")
    print(f"  {hex(s)} ctx={ctx}")

# Dump the most promising: those with GET_APP or 0x704
for s in err_sites:
    ctx_has = False
    for j in range(s - 0x100, s + 0x100, 2):
        mw = movw(j)
        b = bl_target(j)
        if (mw and mw[0] == 0x704) or b in (GET_APP, START_NET):
            ctx_has = True
            break
    if ctx_has:
        dump(s - 0x60, s + 0x40, f"err_{hex(s)}")

# Walk START_NET from entry listing EVERY precondition CMP / LDRB / GET_APP
print("\n=== START_NET precondition inventory ===")
preconds = []
o = START_NET
end = 0x18E8600
while o < end:
    b = bl_target(o)
    if b == GET_APP:
        cmps = []
        j = o + 4
        while j < o + 0x40:
            hw = u16(j)
            if (hw & 0xFF00) == 0x2800:
                cmps.append((j, hw & 0xFF))
            bc = bcond_w(j)
            if bc:
                cmps.append((j, f"Bcond->{hex(bc[0])} c={bc[1]}"))
                break
            # short BEQ/BNE
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm & 0x80:
                    imm -= 256
                tgt = j + 4 + imm * 2
                cmps.append((j, f"B{cond}->{hex(tgt)}"))
                break
            j += 2
        preconds.append(("GET_APP", o, cmps))
    if (u16(o) & 0xFFF0) == 0xF890:
        hw2 = u16(o + 2)
        imm = hw2 & 0xFFF
        if imm in (0xBF4, 0xBF5, 0xBF6, 0xC3, 0x33E, 0x554, 0x83):
            cmps = []
            j = o + 4
            while j < o + 0x20:
                hw = u16(j)
                if (hw & 0xFF00) == 0x2800:
                    cmps.append(hw & 0xFF)
                j += 2
            preconds.append((f"LDRB+{hex(imm)}", o, cmps))
    # CMP imm standalone interesting
    hw = u16(o)
    if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) in (0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 13, 16, 28, 36):
        # only if not already captured
        pass
    if (u16(o) & 0xF800) in (0xE800, 0xF000, 0xF800):
        o += 4
    else:
        o += 2

for kind, site, cmps in preconds:
    print(f"  {kind} @{hex(site)} follow={cmps}")

# After CMP#5 fail BNE.W to 0x19a8484 — search THAT function for MOVS#2 / SIT complete
# Also search for log call with ignore string id near 0x18e8330 fail path
# Re-examine: maybe f040 80a8 is NOT the ignore — check IT block / other
# Look at bytes AFTER successful CMP path vs fail for "Ignored" log id load
print("\n=== strings near continue MOVW 0x9ddc (0x44f39ddc) ===")
# file off for 0x44f39ddc - 0x40000000 = 0x4f39ddc — check
for va in (0x44F39DDC, 0x44F39CD0, 0x44F39D30, 0x44F39D80, 0x44F3A0D0):
    off = va - 0x40000000
    if 0 <= off < len(IMG):
        s = IMG[off : off + 80].split(b"\x00", 1)[0]
        print(f"  {hex(va)} -> {s[:70]!r}")

# Find log-id table: search for pointer to ignore string in data
ign = IMG.find(b"START_NETWORK Ignored: SIM is not ready")
print(f"\nignore @ {hex(ign)}")
# Look for relative offsets used by log system — dump 32 bytes before string in case it's in a table
print("before:", IMG[ign - 32 : ign].hex())

# Who BL→START_NET from functions that MOVS#2+STRB error?
print("\n=== callers of START_NET: dump 0x40 before each for error setup ===")
callers = []
for lo, hi in [(0x1400000, 0x1A00000)]:
    p = lo
    while p + 4 < hi:
        if bl_target(p) == START_NET:
            callers.append(p)
        p += 2
for p in callers:
    # check if function contains MOVS#2 STRB #0xa within ±0x200 of call
    has_err = False
    for j in range(max(0, p - 0x200), min(len(IMG), p + 0x80), 2):
        hw = u16(j)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for k in range(j, j + 0x20, 2):
                if (u16(k) & 0xFFF0) == 0xF880 and (u16(k + 2) & 0xFFF) == 0xA:
                    has_err = True
    print(f"  BL@{hex(p)} has_err2_strb_a={has_err}")
    if has_err:
        dump(p - 0x80, p + 0x20, f"caller_err_{hex(p)}")

# Key insight from live: 0x0704 returns length=12 error=2 — that's the SIT response
# completer packing error into byte 10. Find SIT NET response path for selection auto.
# Search "SIT_NETWORK_SELECTION" string and nearby code xrefs
s = IMG.find(b"SIT_NETWORK_SELECTION")
print(f"\nSIT_NETWORK_SELECTION @ {hex(s)} ctx={IMG[s:s+40]!r}")
# Find MOVW to this string
lo = s & 0xFFFF
# try VA = 0x40000000+s
va = 0x40000000 + s
print(f" hypothetical VA {hex(va)}")

# Dump region_1738 around each MOVW #704
print("\n=== 0x1738 MOVW #704 sites detail ===")
for o in range(0x1738800, 0x1738F80, 2):
    mw = movw(o)
    if mw and mw[0] == 0x704:
        dump(o - 0x30, o + 0x50, f"704@{hex(o)}")

print("\nDONE")
