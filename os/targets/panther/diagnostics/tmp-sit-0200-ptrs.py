#!/usr/bin/env python3
"""Find SIT 0x0200 TX path: log-id tables, STRB@17 after BF4, GetSimStatus builders."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
stream = Path("sit-stream.so").read_bytes()
ril = Path("libsitril.so").read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def bl(o):
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


def movw(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def movt(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    return (
        (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF),
        (hw2 >> 8) & 0xF,
    )


def dump(start, end, lab):
    print(f"\n=== {lab} ===")
    o = start
    while o < end:
        hw = u16(o)
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(o + 2)
            extra = ""
            r, b, t = movw(o), bl(o), movt(o)
            if r:
                extra = f" ;MOVW r{r[1]},#{hex(r[0])}"
            if t:
                extra = f" ;MOVT r{t[1]},#{hex(t[0])}"
            if b is not None:
                extra = f" ;BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF880:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;STRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF890:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDRB.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f" {hex(o)}:{hw:04x} {hw2:04x}{extra}")
            o += 4
        else:
            extra = ""
            if (hw & 0xFF00) == 0x2000:
                extra = f" ;MOVS r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xFF00) == 0x2800:
                extra = f" ;CMP r{(hw>>8)&7},#{hw&0xff}"
            if (hw & 0xF800) == 0x7800:
                extra = f" ;LDRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            if (hw & 0xF800) == 0x7000:
                extra = f" ;STRB r{hw&7},[r{(hw>>3)&7},#{(hw>>6)&0x1f}]"
            print(f" {hex(o)}:{hw:04x}{extra}")
            o += 2


# 1) Find VA of Tx SIM Status in pointer tables (word containing VA)
sva = VA + (img.find(b"Tx SIM Status(app_state=%d") - MAIN)
print(f"TxSIMStatus VA={hex(sva)}")
needle = struct.pack("<I", sva)
ptrs = []
start = 0
while True:
    j = img.find(needle, start)
    if j < 0:
        break
    ptrs.append(j)
    start = j + 4
print(f"ptr table entries: {len(ptrs)} {[hex(p) for p in ptrs[:12]]}")
for p in ptrs[:6]:
    # look at surrounding as possible log table: id, fmt_ptr, ...
    print(f"  around {hex(p)}:")
    for i in range(-4, 5):
        off = p + i * 4
        if 0 <= off < len(img) - 4:
            v = u32(off)
            print(f"    [{i:+d}] {hex(off)} = {hex(v)}")

# 2) Same for sitSendSimStatusChangeInd / SIT_GET_SIM_STATUS
for name in [
    b"sitSendSimStatusChangeInd",
    b"SIT_GET_SIM_STATUS",
    b"Tx SIM Status",
    b"Rx SIM Status",
]:
    j = img.find(name)
    if j < 0:
        print(f"{name}: not found")
        continue
    va = VA + (j - MAIN)
    print(f"\n{name.decode(errors='replace')} off={hex(j)} VA={hex(va)}")
    n = struct.pack("<I", va)
    ps = []
    s = 0
    while True:
        k = img.find(n, s)
        if k < 0:
            break
        ps.append(k)
        s = k + 4
        if len(ps) > 8:
            break
    print(f"  ptrs={len(ps)} {[hex(x) for x in ps]}")

# 3) AP side: find OnGetSimStatusDone / packet offsets in libsitril
# strings around SimStatus
idx = 0
while True:
    j = ril.find(b"OnGetSimStatusDone", idx)
    if j < 0:
        break
    print(f"ril OnGetSimStatusDone @{hex(j)}")
    # nearby printable
    chunk = ril[max(0, j - 64) : j + 128]
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 6:
                print(" ", cur.decode())
            cur = bytearray()
    idx = j + 1

# Search RIL for byte offset comments / GetAppState
for n in [b"GetAppState", b"appState", b"mAppState", b"APP_STATE", b"byte 17", b"+17"]:
    c = ril.count(n)
    if c:
        print(f"ril {n}: {c} @{hex(ril.find(n))}")

# sit-stream ProtocolSimStatusAdapter
for n in [
    b"ProtocolSimStatusAdapter",
    b"InitRequestHeader",
    b"BuildSimGetStatus",
    b"SimStatusAdapter",
]:
    print(f"stream {n}: {stream.find(n)}")
    print(f"ril {n}: {ril.find(n)}")

# 4) In CP: find all LDRB #0xBF4 (already 4) and also LDRB #0xBF6 used near STRB imm5/17
# Broader: after SET_APP STRB BF4, who copies to TX buffer?
# Search STRB.W imm=5 (common payload offset after SIT hdr) with nearby LDRB BF4 within ±0x80
print("\n=== LDRB BF4 proximity to STRB imm 5/17/0x11 ===")
bf4_sites = [0x18EC8EC, 0x199170E, 0x1991788, 0x3708650]
for site in bf4_sites:
    for p in range(max(0, site - 0x100), min(len(img) - 4, site + 0x100), 2):
        hw, hw2 = u16(p), u16(p + 2)
        if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) in (5, 0x11, 17):
            print(f"  near {hex(site)}: STRB.W @{hex(p)} imm={hw2&0xfff}")
        if (hw & 0xF800) == 0x7000 and ((hw >> 6) & 0x1F) in (5, 17):
            print(f"  near {hex(site)}: STRB @{hex(p)} imm={(hw>>6)&0x1f}")

# 5) Search whole image for pattern: LDRB BF4 then within 32 instr STRB to [rx,#5] or #17
print("\n=== global LDRB BF4 + nearby STRB 5/17 (wider) ===")
o = 0x1000000
found = 0
while o < 0x3C00000 - 4:
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF890 and (hw2 & 0xFFF) == 0xBF4:
        for p in range(o, min(o + 0x80, 0x3C00000 - 4), 2):
            h, h2 = u16(p), u16(p + 2)
            if (h & 0xFFF0) == 0xF880 and (h2 & 0xFFF) in (5, 0x11, 17, 0x0F):
                print(f"  {hex(o)} LDRB BF4 -> STRB.W @{hex(p)} imm={h2&0xfff}")
                found += 1
            if (h & 0xF800) == 0x7000 and ((h >> 6) & 0x1F) in (5, 17, 15):
                print(f"  {hex(o)} LDRB BF4 -> STRB @{hex(p)} imm={(h>>6)&0x1f}")
                found += 1
    o += 2
print(f"pairs={found}")

# 6) Search for SIT msg id 0x0200 construction: MOVW rX,#0x200 near SIM status builders
# Common in Shannon: store halfword msgid then fill payload
print("\n=== MOVW #0x200 near LDRB BF4/BF6 (±0x200) ===")
for site in [0x18EC8EC, 0x199170E, 0x1991788, 0x3708650, 0x14FB5C6, 0x14FB380]:
    for p in range(max(0, site - 0x200), min(len(img) - 4, site + 0x200), 2):
        r = movw(p)
        if r and r[0] == 0x200:
            print(f"  MOVW #0x200 @{hex(p)} near {hex(site)}")

# 7) DecodeSimPinSkipReq - find as C string in symbol/debug, find code via "PIN SKIP" table ptr
for name in [
    b"PIN SKIP FAILED: NOT eSIM",
    b"PIN SKIP FAILED: Invalid SimState",
    b"A[USIM_%d] PIN SKIP",
]:
    j = img.find(name)
    va = VA + (j - MAIN)
    n = struct.pack("<I", va)
    ps = []
    s = 0
    while True:
        k = img.find(n, s)
        if k < 0:
            break
        ps.append(k)
        s = k + 4
        if len(ps) > 6:
            break
    print(f"\n{name[:40]!r} VA={hex(va)} ptrs={[hex(x) for x in ps]}")
    for p in ps[:2]:
        for i in range(-2, 3):
            off = p + i * 4
            print(f"  [{i:+d}] {hex(u32(off))}")
