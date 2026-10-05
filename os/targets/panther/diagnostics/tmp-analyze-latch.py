#!/usr/bin/env python3
"""CDMA_TIMING_LATCH_CNF producer + byte+8; FN_B caller event; Present semantics."""
import struct
from pathlib import Path

PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
MAIN_OFF = 0x16C10
END = MAIN_OFF + 0x05917ACC
VA_BASE = 0x40010000
img = PATH.read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va(o):
    return VA_BASE + (o - MAIN_OFF)


def off_va(v):
    return MAIN_OFF + (v - VA_BASE)


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def bl_target(o):
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


def find_str(needle: bytes):
    out = []
    i = 0
    while True:
        j = img.find(needle, i)
        if j < 0:
            break
        a = j
        while a > 0 and 32 <= img[a - 1] < 127:
            a -= 1
        b = j
        while b < len(img) and 32 <= img[b] < 127:
            b += 1
        out.append((a, img[a:b].decode("ascii", "replace")))
        i = j + 1
    return out


def cstr_va(v, n=100):
    o = off_va(v)
    if o < 0 or o >= len(img):
        return None
    b = o
    while b < o + n and 32 <= img[b] < 127:
        b += 1
    return img[o:b].decode("ascii", "replace") if b > o else None


def dump(start, end):
    o = start
    lines = []
    while o < end and o < END - 4:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, t, bt = movw(o), movt(o), bl_target(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if bt is not None:
                extra = f" ;BL->{hex(bt)}"
            if (hw & 0xFFF0) == 0xF880:
                extra = f" ;STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" ;LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF8D0:
                extra = f" ;LDR.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            lines.append(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            elif (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            elif (hw & 0xFFC0) == 0x4600:
                extra = f" ;MOV r{hw&7},r{(hw>>3)&7}"
            elif (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            lines.append(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
    return "\n".join(lines)


def va_xrefs(soff):
    v = va(soff)
    lo, hi = v & 0xFFFF, (v >> 16) & 0xFFFF
    xrefs = []
    for o in range(MAIN_OFF, END - 8, 2):
        r = movw(o)
        if not r or r[0] != lo:
            continue
        for p in range(max(MAIN_OFF, o - 16), min(END, o + 20), 2):
            t = movt(p)
            if t and t[0] == hi and t[1] == r[1]:
                xrefs.append(o)
                break
    return xrefs


# 1) Decode log for CNF — find code that logs rat_mode
print("=== Decode CNF(rat_mode) xrefs ===")
for off, s in find_str(b"Decode MMC_LTEL1_CDMA_TIMING_LATCH_CNF"):
    print(f"@{hex(off)}: {s}")
    # A[] logs use id packing not VA — search nearby for 0x5xx style via litpool of VA
    v = va(off)
    nb = struct.pack("<I", v)
    pools = []
    i = 0
    while True:
        j = img.find(nb, i)
        if j < 0:
            break
        pools.append(j)
        i = j + 1
    print(f"  litpools={list(map(hex, pools[:6]))}")
    xrefs = va_xrefs(off)
    print(f"  MOVW+MOVT xrefs={list(map(hex, xrefs[:10]))} n={len(xrefs)}")

# real_log heuristic: find sites that load string via known decode helper near L1LC
# Search for LDRB [rN,#8] near string mention / near handler 0x14c388e already known
print("\n=== Handler 0x14c388e: all LDRB #8 / rat-related ===")
print(dump(0x14C388E, 0x14C3990))

# 2) Who SENDS CNF — look for Send / build strings
print("\n=== Send/build CNF / REQ strings ===")
for needle in (
    b"Send LTEL1_MMC_CDMA_TIMING_LATCH_REQ",
    b"CDMA_TIMING_LATCH_REQ to EVDO",
    b"CDMA_TIMING_LATCH_CNF",
    b"Send.*TIMING_LATCH_CNF",
    b"L1C_MMC_CDMA_TIMING_LATCH_REQ_Handler",
    b"TIMING_LATCH_CNF(rat_mode",
    b"rat_mode",
):
    if b".*" in needle:
        continue
    for off, s in find_str(needle)[:6]:
        print(f"  @{hex(off)}: {s[:120]}")

# Find format with rat_mode specifically for LATCH
print("\n=== rat_mode near LATCH ===")
for off, s in find_str(b"rat_mode"):
    if "LATCH" in s or "CDMA" in s or "Timing" in s or "TIMING" in s:
        print(f"  @{hex(off)}: {s[:120]}")

# 3) REQ handler 0x286e6f4 (string) — find code
print("\n=== L1C_MMC_CDMA_TIMING_LATCH_REQ_Handler string nearby ===")
for off, s in find_str(b"L1C_MMC_CDMA_TIMING_LATCH_REQ_Handler"):
    print(f"@{hex(off)} VA={hex(va(off))}")
    # dump surrounding function names
    region = img[max(0, off - 0x100) : off + 0x200]
    cur = b""
    base = max(0, off - 0x100)
    for i, b in enumerate(region):
        if 32 <= b < 127:
            cur += bytes([b])
        else:
            if len(cur) >= 10 and (b"LATCH" in cur or b"Handler" in cur or b"MMC" in cur):
                print(f"  @{hex(base+i-len(cur))}: {cur.decode()}")
            cur = b""

# 4) Find who stores to msg+8 when building CNF — search STRB #8 near TIMING_LATCH send paths
# First: name table VA for MMC_LTEL1_CDMA_TIMING_LATCH_CNF used as TX
cnf_name = find_str(b"MMC_LTEL1_CDMA_TIMING_LATCH_CNF")[0][0]
print(f"\nCNF name @{hex(cnf_name)} VA={hex(va(cnf_name))}")
# litpool
v = va(cnf_name)
pools = []
i = 0
nb = struct.pack("<I", v)
while True:
    j = img.find(nb, i)
    if j < 0:
        break
    pools.append(j)
    i = j + 1
print(f"litpools={list(map(hex, pools))}")

# Also MMC_L1C_CDMA_TIMING_LATCH_CNF (producer from L1C side?)
for off, s in find_str(b"MMC_L1C_CDMA_TIMING_LATCH_CNF"):
    print(f"alt name @{hex(off)} VA={hex(va(off))}")

# 5) FN_B sole caller 0x14c5fe6 — which MMC message?
print("\n=== FN_B caller 0x14c5fe6 dispatcher context ===")
print(dump(0x14C5F80, 0x14C6020))
# Find BL sites to 0x14c5fe6
cs = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == 0x14C5FE6]
print(f"BL 0x14c5fe6: {list(map(hex, cs))}")
# Or 0x14c5fe6 IS the BL to FN_B — find function start and dispatcher case name
fn = None
for p in range(0x14C5FE6, 0x14C5FE6 - 0x200, -2):
    if (u16(p) & 0xFFF0) == 0xE92D or u16(p) in (0xB5F0, 0xB5B0, 0xB570):
        fn = p
        break
print(f"fn containing BL FN_B: {hex(fn) if fn else None}")
if fn:
    print(dump(fn, min(fn + 0x100, 0x14C5FE6 + 0x20)))

# Dispatcher that calls 0x14c5fe6's function — search BL to fn
if fn:
    callers = [o for o in range(MAIN_OFF, END - 4, 2) if bl_target(o) == fn]
    print(f"callers of {hex(fn)}: {list(map(hex, callers[:10]))}")
    for c in callers[:3]:
        print(f"\n-- caller {hex(c)} --")
        # look for MOVW r1 message name 0x4106xxxx nearby
        for o in range(c - 0x40, c + 0x10, 2):
            r = movw(o)
            t = None
            if r:
                for p in range(o - 16, o + 20, 2):
                    tt = movt(p)
                    if tt and tt[1] == r[1] and 0x4100 <= tt[0] <= 0x4110:
                        t = tt
                        break
            if r and t:
                v = (t[0] << 16) | r[0]
                print(f"  {hex(o)}: VA={hex(v)} -> {cstr_va(v)}")

# Broader: around 0x14c5fe6 in same switch style as 0x14b79e2
print("\n=== Switch-style name ptrs near 0x14b7xxx if FN_B wrapper called from there ===")
# Search BL to fn(0x14c5xxx) from 0x14b7000..0x14b9000
for o in range(0x14B7000, 0x14B9000, 2):
    bt = bl_target(o)
    if bt and 0x14C5E00 <= bt <= 0x14C6200:
        # get preceding name
        name = None
        for p in range(o - 0x30, o, 2):
            r = movw(p)
            if not r:
                continue
            for q in range(p - 16, p + 20, 2):
                t = movt(q)
                if t and t[1] == r[1] and t[0] in (0x4106, 0x4100, 0x4107, 0x4108):
                    name = cstr_va((t[0] << 16) | r[0])
        print(f"  {hex(o)} BL->{hex(bt)} name={name}")

# 6) Present semantics: STATUS mapping already known
print("\n=== STATUS Present→app_state (confirm) ===")
print(dump(0x14FB350, 0x14FB5E0))

# 7) Who fills CNF payload — EVDO path
print("\n=== EVDO / Send CNF builders ===")
for needle in (
    b"CDMA_TIMING_LATCH_REQ to EVDO",
    b"TIMING_LATCH_CNF to",
    b"Send MMC_L1C_CDMA_TIMING_LATCH_CNF",
    b"MMC_L1C_CDMA_TIMING_LATCH_CNF",
    b"BuildCdmaTimingLatch",
    b"TimingLatchCnf",
    b"timing_latch",
):
    for off, s in find_str(needle)[:4]:
        print(f"  @{hex(off)}: {s[:120]}")

print("\nDONE")
