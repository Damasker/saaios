#!/usr/bin/env python3
"""Adversarial RO: VerifyPin RSP → SET_APP#5 / Present / +0xBF6."""
from __future__ import annotations
import struct
from collections import deque
from pathlib import Path

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA0 = 0x40010000

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

def find_str(s: bytes):
    out = []
    start = 0
    while True:
        i = IMG.find(s, start)
        if i < 0:
            break
        out.append(i)
        start = i + 1
    return out

# Known anchors
FN_A = 0x14F692C
FN_A_P2 = 0x14F6A14
STATUS_BF6 = 0x14FB380
SET_APP = 0x19916D2
SET_APP5 = 0x14FB5C6
STATUS_ENTRY = 0x14FB322
STATUS_WRAP = 0x14C6626
GETOBJ = 0x20EA040

LABELS = {
    FN_A: "FN_A",
    FN_A_P2: "FN_A_Present2",
    STATUS_BF6: "STATUS_STRB_+0xBF6",
    SET_APP: "SET_APP",
    SET_APP5: "SET_APP5_site",
    STATUS_ENTRY: "STATUS",
    STATUS_WRAP: "STATUS_WRAP",
    GETOBJ: "getobj",
    0x14FB404: "SET_APP2_PIN_site",
    0x14FB5C0: "CMP_BF6_eq2",
    0x14FB432: "STRB_+0xBF5_Pin1V",
}

def near_label(addr):
    for a, n in LABELS.items():
        if abs(addr - a) < 8:
            return n
    return None

# 1) Find log strings
needles = [
    b"First PIN1 Verification is done",
    b"NS_USIM_VERIFYPIN_RSP",
    b"Rx NS_USIM_VERIFYPIN_RSP",
    b"sitSendNsSimInfoReq",
    b"SIM STATUS update: Present",
]
print("=== strings ===")
str_hits = {}
for n in needles:
    hits = find_str(n)
    str_hits[n] = hits
    print(f"  {n.decode(errors='replace')[:50]!r}: {[hex(h) for h in hits[:6]]}")

# Find MOVW log-id xrefs by scanning for ADR to string pools is hard;
# use previously known FirstPIN sites + find xrefs via literal pools near strings.

# 2) Dump First PIN sites (known) + scan forward for STRB/BL
FIRST_PIN = [0x1F0456C, 0x1F0469A]
VERIFYPIN_RX = []  # discover via string xref heuristic

def dump_range(start, end, lab, maxn=80):
    print(f"\n=== {lab} [{hex(start)}..{hex(end)}] ===")
    o = start
    n = 0
    while o < end and n < maxn:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b is not None:
                lb = near_label(b) or LABELS.get(b)
                extra = f" BL->{hex(b)}" + (f"={lb}" if lb else "")
            if (hw & 0xFFF0) == 0xF880:
                extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFF00) == 0x2000:
                pass
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7000:
                extra = f" STRB [r{hw&7},#{(hw>>6)&0x1f}]"
            b = None
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                tgt = o + 4 + imm * 2
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14:
                    extra = f" B{names[cond]}->{hex(tgt)}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400: imm -= 0x800
                extra = f" B->{hex(o+4+imm*2)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2
        n += 1

# 3) BFS from First PIN sites
print("\n=== BFS from First PIN1 Verification sites ===")
seeds = list(FIRST_PIN)
# Also add sites that BL to FirstPIN blocks - find callers of those addrs
for s in FIRST_PIN:
    dump_range(s - 0x40, s + 0xC0, f"around FirstPIN {hex(s)}", maxn=60)

interesting_hits = []
visited_fn = set()
q = deque()
for s in FIRST_PIN:
    q.append((s, 0, f"FirstPIN@{hex(s)}"))

MAX_DEPTH = 4
MAX_FN = 80
fns_seen = 0

while q and fns_seen < MAX_FN:
    start, depth, path = q.popleft()
    # normalize to function-ish window
    fn = start & ~1
    if fn in visited_fn:
        continue
    visited_fn.add(fn)
    fns_seen += 1
    end = min(fn + 0x200, len(IMG) - 4)
    o = fn
    while o < end - 4:
        # STRB.W #0xBF6 / #0xBF5 / #0 / #20
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            if (hw & 0xFFF0) == 0xF880:
                off = hw2 & 0xFFF
                rn = hw & 0xF
                if off in (0xBF6, 0xBF5, 0xBF4, 0x14, 0x20, 0):
                    interesting_hits.append((path, depth, o, f"STRB.W [r{rn},#{hex(off)}]"))
            b = bl_target(o)
            if b is not None:
                lb = LABELS.get(b) or near_label(b)
                if lb or b in (FN_A, FN_A_P2, STATUS_BF6, SET_APP, SET_APP5, STATUS_ENTRY, STATUS_WRAP, GETOBJ):
                    interesting_hits.append((path, depth, o, f"BL->{hex(b)}={lb}"))
                # enqueue callees one level
                if depth < MAX_DEPTH and b not in visited_fn and MAIN < b < len(IMG) - 0x100:
                    # skip huge jumps to libc-ish
                    if abs(b - fn) < 0x800000:
                        q.append((b, depth + 1, path + f" ->{hex(b)}"))
            o += 4
        else:
            # MOVS #2 then nearby STRB
            if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
                # look ahead 16 bytes for STRB
                for a in range(o, min(o + 16, end - 2), 2):
                    h2 = u16(a)
                    if (h2 & 0xF800) == 0x7000:
                        interesting_hits.append((path, depth, o, f"MOVS#2 + STRB@{hex(a)}"))
                    if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800) and a + 4 <= end:
                        if (h2 & 0xFFF0) == 0xF880:
                            interesting_hits.append((path, depth, o, f"MOVS#2 + STRB.W@{hex(a)} off=#{hex(u16(a+2)&0xfff)}"))
            o += 2

print(f"\nBFS fns={fns_seen} interesting={len(interesting_hits)}")
for path, depth, o, desc in interesting_hits[:80]:
    print(f"  d{depth} {hex(o)}: {desc}  [{path[:80]}]")

# 4) Re-prove SET_APP#5 gate: dump 0x14fb5a0..0x14fb5e0 and who writes r0 before CMP
print("\n=== SET_APP#5 gate dump ===")
dump_range(0x14FB580, 0x14FB5E0, "READY gate", maxn=40)

# All STRB.W #0xBF6 in image
print("\n=== all STRB.W #0xBF6 ===")
bf6 = []
o = MAIN
while o < len(IMG) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) and (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
        bf6.append(o)
        o += 4
    else:
        o += 2 if (hw & 0xF800) not in (0xE800, 0xF000, 0xF800) else 4
print([hex(x) for x in bf6])

# All SET_APP BL with MOVS r0,#5 nearby
print("\n=== MOVS r0,#5 near BL SET_APP ===")
o = MAIN
set5 = []
while o < len(IMG) - 8:
    hw = u16(o)
    if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 5 and ((hw >> 8) & 7) == 0:
        # MOVS r0,#5
        for a in range(o, min(o + 12, len(IMG) - 4), 2):
            b = bl_target(a)
            if b == SET_APP:
                set5.append((o, a))
    o += 2
print([(hex(a), hex(b)) for a, b in set5])

# 5) PresentObj[0]=2: MOVS #2; STRB [rn,#0] within FN_A and globally near getobj #636c
print("\n=== Present=2 STRB scan near getobj #0x636c ===")
# find MOVW #0x636c
sites_636c = []
o = MAIN
while o < len(IMG) - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
        # MOVW encoding F240
        if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
            i = (hw >> 10) & 1
            imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
            if imm == 0x636C:
                sites_636c.append(o)
        o += 4
    else:
        o += 2
print("MOVW #636c count", len(sites_636c), [hex(x) for x in sites_636c[:12]])

# For each, scan +0x80 for MOVS#2 + STRB #0
for s in sites_636c:
    window = IMG[s:s+0x120]
    # simple: look for 2002 (MOVS r0,#2) or 2102 etc then F880 with off 0
    found = False
    for off in range(0, len(window) - 4, 2):
        abs_o = s + off
        hw = u16(abs_o)
        if (hw & 0xFF00) == 0x2000 and (hw & 0xFF) == 2:
            for off2 in range(off, min(off + 24, len(window) - 4), 2):
                h2 = u16(s + off2)
                if (h2 & 0xF800) in (0xE800, 0xF000, 0xF800):
                    h3 = u16(s + off2 + 2)
                    if (h2 & 0xFFF0) == 0xF880 and (h3 & 0xFFF) == 0:
                        print(f"  Present2 candidate @{hex(abs_o)} STRB.W@{hex(s+off2)} near getobjlit @{hex(s)}")
                        found = True
    if not found and s in (0x14F69E0, 0x14F6A00) or (0x14F6900 <= s <= 0x14F6B00):
        print(f"  (FN_A region lit @{hex(s)})")

print("\nDONE")
