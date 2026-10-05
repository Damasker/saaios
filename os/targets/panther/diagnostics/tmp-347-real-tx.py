#!/usr/bin/env python3
"""Dump real Tx SIM Status (0x347) call sites with r0/r1; find SIT packet fill."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
stream = Path("sit-stream.so").read_bytes()


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


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


# r0/r1 MOVW #0x347 from prior list
sites = [
    0x11F5470,
    0x1609B5A,
    0x1697A08,
    0x1750C70,
    0x1B9ACBC,
    0x1BA1C5C,
    0x2056610,
    0x2A29602,
]
for s in sites:
    r = movw(s)
    if not r:
        print(f"no movw at {hex(s)}")
        continue
    dump(s - 0x50, s + 0x40, f"347_r{r[1]}@{hex(s)}")

# sitSendSimStatusChangeInd - find similar log registration then real call
# Search MOVW # of ids near sitSend string registration
# Look in SIT0 region 0x1d00000 for GET_APP then store to msg+5
print("\n=== SIT region 0x1dxxxx: BL GET_APP already known; expand 0x1d67a00 ===")
dump(0x1D67900, 0x1D67B00, "SIT_cluster")

# Search for sitSend function via "sitSendSimStatusChangeInd" as C identifier
# used in assert/log - may be MOVW of a different log id
j = img.find(b"sitSendSimStatusChangeInd")
print(f"\nsitSendSimStatusChangeInd @{hex(j)}")
# print nearby C identifiers
a = max(0, j - 400)
chunk = img[a : j + 80]
cur = bytearray()
for b in chunk:
    if 32 <= b < 127:
        cur.append(b)
    else:
        if len(cur) >= 8:
            print(" ", cur.decode())
        cur = bytearray()

# AP: ProtocolSimStatusAdapter Init - find in binary and look at offsets
# Search for byte pattern that loads offset 17 / 0x11
idx = stream.find(b"ProtocolSimStatusAdapter")
print(f"\nProtocolSimStatusAdapter @{hex(idx)}")
# Find Init method mangled
for n in [
    b"_ZN25ProtocolSimStatusAdapter4InitE",
    b"ProtocolSimStatusAdapter::Init",
    b"m_appStatus",
    b"appStatus",
]:
    print(f"  {n}: {stream.find(n)}")

# Disassemble stream around BuildSimGetStatus to confirm request
# For response parsing: search libsitril / stream for +0x11 or offset 17 stores into AppState
# In ELF .text of stream - use simple scan for MOV #0x11 near SimStatus
print("\n=== stream: look for app state field comments ===")
for n in [
    b"app_state",
    b"AppState",
    b"APPSTATE_PIN",
    b"APPSTATE_READY",
    b"RIL_APPSTATE",
    b"pin1",
    b"PinState",
]:
    c = stream.count(n)
    print(f"stream {n}: {c}")

# In CP: find function that copies SIM object fields into SIT buffer
# Pattern: LDRB BF4, STRB to [rX, #5] where #5 is payload offset after 12-byte hdr
# (byte17 absolute = hdr12 + 5). Search STRB.W imm=5 after any LDRB BF4 within same fn - already 0.
# Maybe they use offset from a base pointer that already points into payload:
# STRB rt, [rN, #0] after ADD rN, rBuf, #17
# Search ADD #17 / #0x11 near GET_APP callers in SIT

print("\n=== ADD/MOV #0x11 or #17 near SIT GET_APP caller 0x1d67a34 ===")
dump(0x1D67800, 0x1D67C00, "wide_SIT")

# Alternate theory: SIT response built in 0x1f0xxxx VerifyPin / SimInfo path
# Search MOVW #0x200 (msgid) in 0x1d00000-0x1f80000 more carefully with STRH follow
print("\n=== MOVW #0x200 then STRH within 16 instr (SIT) ===")
o = 0x1D00000
hits = []
while o < 0x1F80000 - 4:
    r = movw(o)
    if r and r[0] == 0x200:
        rd = r[1]
        for p in range(o + 4, min(o + 0x30, 0x1F80000 - 2), 2):
            hw = u16(p)
            # STRH rt,[rn,#imm] T1: 0x8000 | imm5<<6 | rn<<3 | rt
            if (hw & 0xF800) == 0x8000 and (hw & 7) == rd:
                hits.append((o, p, hw))
            # STRH.W
            if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
                hw2 = u16(p + 2)
                if (hw & 0xFFF0) == 0xF8A0:  # STRH.W
                    rt = (hw2 >> 12) & 0xF
                    if rt == rd:
                        hits.append((o, p, hw2 & 0xFFF))
        if len(hits) > 30:
            break
    o += 2
print(f"hits={len(hits)}")
for h in hits[:15]:
    print(f"  MOVW@{hex(h[0])} STR@{hex(h[1])} info={h[2] if not isinstance(h[2],int) or h[2]<0x10000 else hex(h[2])}")
    dump(h[0] - 0x20, h[0] + 0x50, f"sit200@{hex(h[0])}")
