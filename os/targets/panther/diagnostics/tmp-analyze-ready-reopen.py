#!/usr/bin/env python3
"""Re-open READY: LATCH_REQ senders; SET_APP#5; Present=2 writers; CDMA feature."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN, END, VA = 0x16C10, 0x16C10 + 0x05917ACC, 0x40010000
SET_APP = 0x19916D2
FN_A = 0x14F692C


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def va(o):
    return VA + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA)


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


def find(n: bytes):
    out, i = [], 0
    while True:
        j = img.find(n, i)
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
    if not (0 <= o < len(img)):
        return None
    b = o
    while b < o + n and 32 <= img[b] < 127:
        b += 1
    return img[o:b].decode() if b > o else None


# --- 1) SET_APP by imm ---
print("=== SET_APP callers by imm ===")
by = {}
for o in range(MAIN, END - 4, 2):
    if bl(o) != SET_APP:
        continue
    imm = None
    for p in range(max(MAIN, o - 24), o, 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2000 and ((hw >> 8) & 7) == 0:
            imm = hw & 0xFF
        r = movw(p)
        if r and r[1] == 0 and r[0] <= 10:
            imm = r[0]
    by.setdefault(imm, []).append(o)
for imm, sites in sorted(by.items(), key=lambda x: (x[0] is None, x[0] or -1)):
    print(f"  imm={imm}: n={len(sites)} {list(map(hex, sites[:15]))}")

print("\n=== READY #5 site contexts ===")
for o in by.get(5, []):
    # dump nearby CMP/LDRB BF6
    print(f"-- {hex(o)} --")
    for p in range(o - 0x40, o + 4, 2):
        hw = u16(p)
        note = ""
        if (hw & 0xF800) in (0xE800, 0xF000, 0xF800):
            hw2 = u16(p + 2)
            r, b = movw(p), bl(p)
            if r:
                note = f" MOVW r{r[1]},#{hex(r[0])}"
            if b:
                note = f" BL->{hex(b)}"
            if (hw & 0xFFF0) == 0xF890:
                note = f" LDRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            if (hw & 0xFFF0) == 0xF880:
                note = f" STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#{hex(hw2&0xfff)}]"
            print(f"  {hex(p)}:{note or f' {hw:04x}{hw2:04x}'}")
            continue
        if (hw & 0xFF00) == 0x2000:
            note = f" MOVS r{(hw>>8)&7},#{hw&0xff}"
        elif (hw & 0xFF00) == 0x2800:
            note = f" CMP r{(hw>>8)&7},#{hw&0xff}"
        if note:
            print(f"  {hex(p)}:{note}")

# --- 2) All FN_A callers ---
print("\n=== FN_A callers ===")
cs = [o for o in range(MAIN, END - 4, 2) if bl(o) == FN_A]
print(list(map(hex, cs)))

# --- 3) STRB #0xBF6 anywhere ---
print("\n=== STRB.W #0xBF6 ===")
for o in range(MAIN, END - 4, 2):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0xBF6:
        print(f"  {hex(o)} STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#0xbf6]")

# --- 4) Present=2: MOVS #2 + STRB.W [rn,#0] in SIM-ish + other ---
print("\n=== MOVS#2 then STRB.W [*,#0] candidates (SIM+USIM ranges) ===")
ranges = [(0x14F0000, 0x1508000), (0x1980000, 0x19A0000), (0x18B0000, 0x1900000)]
for lo, hi in ranges:
    for o in range(lo, hi, 2):
        if u16(o) != 0x2002:  # MOVS r0,#2
            continue
        for p in range(o, min(o + 16, hi), 2):
            hw, hw2 = u16(p), u16(p + 2)
            if (hw & 0xFFF0) == 0xF880 and (hw2 & 0xFFF) == 0:
                print(f"  {hex(o)}->{hex(p)} STRB.W r{(hw2>>12)&0xf},[r{hw&0xf},#0]")
                break

# --- 5) CDMA feature / unsupported ---
print("\n=== CDMA support / skip strings ===")
for n in (
    b"CDMA not support",
    b"CDMA NOT SUPPORT",
    b"Cdma not support",
    b"No CDMA",
    b"CDMA disabled",
    b"IsCdmaSupport",
    b"isCdmaSupport",
    b"CDMA_SUPPORT",
    b"SupportCdma",
    b"FEATURE_CDMA",
    b"Remove CDMA",
    b"remove CDMA",
    b"CDMA IRAT is not",
    b"skip CDMA",
    b"Skip CDMA",
    b"CDMA RAT is not supported",
    b"UE_RAT_MODE_CAPABILITY",
):
    for o, s in find(n)[:4]:
        print(f"  @{hex(o)}: {s[:110]}")

# --- 6) Send LATCH_REQ: find nearby MOVW of format via lit / approximate by searching
# code that references the send string via A[] packing is hard; look for
# function names
print("\n=== LATCH send / IRAT CDMA proc names ===")
for n in (
    b"L1LC_IratProcCdma",
    b"IratProcCdmaTimingLatch",
    b"IratCdmaTimingLatch",
    b"SendCdmaTimingLatch",
    b"CdmaTimingLatchReq",
    b"gL1LC_IratCdma",
):
    for o, s in find(n)[:8]:
        print(f"  @{hex(o)}: {s[:110]}")

# Preferred table numeric values in sit-stream (look for adjacent enum order)
print("\n=== sit-stream preferred name cluster ===")
data = Path("sit-stream.so").read_bytes()
# find LTE_ONLY and neighbors
j = data.find(b"SIT_NET_PREF_NET_TYPE_LTE_ONLY")
print("LTE_ONLY@", hex(j))
# dump nearby C strings
region = data[max(0, j - 0x400) : j + 0x400]
cur = b""
base = max(0, j - 0x400)
for i, b in enumerate(region):
    if 32 <= b < 127:
        cur += bytes([b])
    else:
        if cur.startswith(b"SIT_NET_PREF_NET_TYPE_"):
            print(f"  @{hex(base+i-len(cur))}: {cur.decode()}")
        cur = b""

print("\nDONE")
