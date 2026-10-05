#!/usr/bin/env python3
"""RO: GapMeasurePause vs SIM READY / START_NETWORK / PIN; early Present=2 at init."""
import struct
from pathlib import Path
from collections import deque

IMG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin").read_bytes()
SCAN_LO, SCAN_HI = 0x100000, min(len(IMG) - 4, 0x5A00000)
VA_BASE, MAIN_OFF = 0x40010000, 0x16C10
GET_APP, SET_APP, FN_A = 0x18EC8C0, 0x19916D2, 0x14F692C
STATUS, SET6 = 0x14FB322, 0x14C643C

def u16(o): return struct.unpack_from("<H", IMG, o)[0]
def u32(o): return struct.unpack_from("<I", IMG, o)[0]

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

def dump(start, nbytes=0xA0, lab=""):
    print(f"\n=== {lab} @ {hex(start)} ===")
    o, end = start, start + nbytes
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            b = bl_target(o)
            if b: extra = f" BL->{hex(b)}"
            if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
                i = (hw >> 10) & 1
                imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
                extra = f" MOVW r{(hw2>>8)&0xf},#{hex(imm)}"
            if (hw & 0xFFF0) == 0xF890: extra = f" LDRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF880: extra = f" STRB.W [r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f"  {hex(o)}: {hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000: extra = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800: extra = f" CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF000) == 0xD000:
                cond = (hw >> 8) & 0xF
                imm = hw & 0xFF
                if imm >= 0x80: imm -= 0x100
                names = "EQ NE CS CC MI PL VS VC HI LS GE LT GT LE".split()
                if cond < 14: extra = f" B{names[cond]}->{hex(o+4+imm*2)}"
            if (hw & 0xF800) == 0xE000:
                imm = hw & 0x7FF
                if imm >= 0x400: imm -= 0x800
                extra = f" B->{hex(o+4+imm*2)}"
            print(f"  {hex(o)}: {hw:04x}{extra}")
            o += 2

# --- find GetIsGapMeasurePause / post PAUSE_REQ code ---
print("=== GapMeasurePause / READY / SIM / START_NETWORK strings ===")
for n in [
    b"GetIsGapMeasurePause",
    b"GapMeasurePause",
    b"SADR_GAP_MEASURE_PAUSE_REQ",
    b"START_NETWORK",
    b"StartNetwork",
    b"SIM not ready",
    b"SimNotReady",
    b"app_state",
    b"AppState",
    b"PIN state",
    b"before SIM",
    b"SIM READY",
    b"not READY",
]:
    idx, c = 0, 0
    while c < 5:
        j = IMG.find(n, idx)
        if j < 0: break
        s0 = j
        while s0 > 0 and 32 <= IMG[s0-1] < 127: s0 -= 1
        end = IMG.find(b"\x00", j)
        print(f"  {hex(s0)}: {IMG[s0:min(end,s0+100)]!r}")
        idx, c = j + 1, c + 1

# Find code near GetIsGapMeasurePause string @0x5073416 area - in DATA, find xrefs via nearby unique
# Search MOVW log ids near RSM GapMeasure strings in code by finding BL that load string VA

# Locate function that posts PAUSE_REQ: search ADR/MOVW to consumer-side is wrong;
# Post side string VA 0x42614158 (0x261ad68)
POST_STR = 0x261AD68
va = VA_BASE + (POST_STR - MAIN_OFF)
print(f"\nPOST_STR VA {hex(va)}")

# Broader: find "SR_IF ==> LTE_L1LC] MMCIF_L1LC_SADR_GAP" code that logs then sends
# Search for MOVW of distinctive immediates near 0x261ad68 - use find of relative ADR in 0x25xxxxx-0x27xxxxx code?

# Scan for LDR literal pools containing VA of POST_STR
print("\n=== u32 literals to POST_STR VA / file ===")
count = 0
o = 0
post_va = VA_BASE + (POST_STR - MAIN_OFF)
while o < len(IMG) - 4 and count < 25:
    w = u32(o)
    if w == post_va or w == POST_STR:
        print(f"  @{hex(o)}")
        count += 1
    o += 4

# Find START_NETWORK gate on GET_APP - known soft-lock: needs {1,4,5}
print("\n=== START_NETWORK / GET_APP gates (string xrefs) ===")
for n in [b"START_NETWORK", b"Start Network", b"start_network", b"NS_START_NETWORK"]:
    idx = 0
    while True:
        j = IMG.find(n, idx)
        if j < 0: break
        s0 = j
        while s0 > 0 and 32 <= IMG[s0-1] < 127: s0 -= 1
        end = IMG.find(b"\x00", j)
        s = IMG[s0:end]
        if b"SIM" in s or b"App" in s or b"READY" in s or b"PIN" in s or b"state" in s.lower() or b"GET_APP" in s:
            print(f"  {hex(s0)}: {s[:120]!r}")
        idx = j + 1

# Search CMP GET_APP results near known START_NETWORK handlers - find BL GET_APP then CMP #5/#1/#4/#2
print("\n=== BL GET_APP then CMP #1/#2/#4/#5 (sample START-like) ===")
o = SCAN_LO
samples = []
while o < SCAN_HI - 16 and len(samples) < 80:
    if bl_target(o) == GET_APP:
        # look ahead 12 instr for CMP #1/#2/#4/#5
        cmps = []
        a = o + 4
        for _ in range(8):
            h = u16(a)
            if (h & 0xFF00) == 0x2800:
                cmps.append(h & 0xFF)
            if (h & 0xF800) in (0xE800, 0xF000, 0xF800):
                a += 4
            else:
                a += 2
        if any(c in (1, 2, 4, 5, 6, 7) for c in cmps):
            samples.append((o, cmps))
    o += 2
# cluster
from collections import Counter
pat = Counter(tuple(s[1]) for s in samples)
print(f"  patterns: {pat.most_common(15)}")
print(f"  sites with CMP including 5: {sum(1 for s in samples if 5 in s[1])}")
print(f"  sites with CMP 2 only-ish: {[hex(s[0]) for s in samples if s[1][:2]==[2] or s[1]==[2]][:10]}")

# GapMeasure: find functions referencing both GapMeasure and GET_APP / SET_APP / 0x18ec8c0
print("\n=== BFS from SET#6 backward: does 0x18c2acc / SET6 check GET_APP? ===")
dump(0x18C2ACC, 0x90, "SET#6 gate 0x18c2acc")
dump(0x14C643C, 0x80, "SET#6 entry")

# Does dispatcher 0x14b7074 check GET_APP / SIM before accepting PAUSE?
print("\n=== BL GET_APP / SET_APP / FN_A inside dispatcher 0x14b7074..0x14b7d00 ===")
o = 0x14B7074
while o < 0x14B7D00:
    b = bl_target(o)
    if b in (GET_APP, SET_APP, FN_A, 0x18C2ACC, SET6, STATUS):
        print(f"  {hex(o)} -> {hex(b)}")
    o += 2

# Early Present=2: at each getobj#636c, dump first 0x100 for STRB [*,#0] with imm 0/1/2/3
print("\n=== getobj#636c init stores to [obj,#0] ===")
getobj_sites = []
o = SCAN_LO
while o < SCAN_HI - 8:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) == 0xF240 and not (hw2 & 0x8000):
        i = (hw >> 10) & 1
        imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
        if imm == 0x636C:
            for a in range(o, min(o + 0x40, SCAN_HI - 4), 2):
                if bl_target(a) == 0x20EA040:
                    getobj_sites.append((o, a))
                    break
    o += 2

for movw, bl in getobj_sites:
    stores = []
    # scan function-ish 0x200 after BL
    a = bl + 4
    end = bl + 0x200
    while a < end:
        h = u16(a)
        if (h & 0xFF00) == 0x2000 and (h & 0xFF) <= 4:
            rd = (h >> 8) & 7
            for b in range(a + 2, a + 12, 2):
                h2 = u16(b)
                if (h2 & 0xF800) == 0x7000 and (h2 & 7) == rd and ((h2 >> 6) & 0x1F) == 0:
                    stores.append((h & 0xFF, a, b))
                if (h2 & 0xFFF0) == 0xF880:
                    h3 = u16(b + 2)
                    if ((h3 >> 12) & 0xF) == rd and (h3 & 0xFFF) == 0:
                        stores.append((h & 0xFF, a, b))
        if (h & 0xFF00) == 0xBD00 or h == 0x4770:
            break
        if (h & 0xF800) in (0xE800, 0xF000, 0xF800):
            a += 4
        else:
            a += 2
    print(f"  getobj @{hex(movw)}: stores[0]={stores[:10]}")

print("DONE")
