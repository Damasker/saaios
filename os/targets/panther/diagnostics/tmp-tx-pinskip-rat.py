#!/usr/bin/env python3
"""Deep dump SIT status pack candidates; PinSkip eSIM gate; RatMap write paths."""
import struct
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
MAIN = 0x16C10
VA = 0x40010000
stream = Path("sit-stream.so").read_bytes()


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
            if (hw & 0xFFF0) == 0xF8D0:
                rt = (hw2 >> 12) & 0xF
                extra = f" ;LDR.W r{rt},[r{hw&0xf},#{hex(hw2&0xfff)}]"
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


# Primary pack candidates
dump(0x18E8028, 0x18E8600, "TX_CAND_18e80")
dump(0x18C6400, 0x18C6700, "TX_CAND_18c64")

# 0x20e184e — notify helper used by SET_APP with (type=2, app_state)
dump(0x20E184E, 0x20E1A00, "NOTIFY_20e184e")

# Who calls notify with type related to SIM status?
print("\n=== BL→0x20e184e from SIT/SIM windows ===")
for lo, hi in [(0x14F0000, 0x1520000), (0x18C0000, 0x1A00000), (0x1D00000, 0x1F80000)]:
    o = lo
    n = 0
    while o < hi - 4:
        if bl(o) == 0x20E184E:
            # check preceding MOVS r0,#imm
            prev = []
            for p in range(max(lo, o - 0x20), o, 2):
                hw = u16(p)
                if (hw & 0xFF00) == 0x2000:
                    prev.append((p, (hw >> 8) & 7, hw & 0xFF))
            print(f"  {hex(o)} prev_MOVS={prev[-3:]}")
            n += 1
            if n > 15:
                break
        o += 2

# PinSkip: find handler via NS_SIM_PIN_SKIP string
for n in [
    b"NS_SIM_PIN_SKIP",
    b"SIM_PIN_SKIP",
    b"PinSkipReq",
    b"PIN_SKIP_REQ",
    b"DecodeSimPinSkip",
]:
    j = img.find(n)
    print(f"{n}: {hex(j) if j>=0 else None}")
    if j and j > 0:
        a = max(0, j - 100)
        chunk = img[a : j + 60]
        cur = bytearray()
        for b in chunk:
            if 32 <= b < 127:
                cur.append(b)
            else:
                if len(cur) >= 6:
                    print(" ", cur.decode())
                cur = bytearray()

# Search for eSIM check: strings "eSIM" / "isEsim" / "IsEsim" near USIM
for n in [b"isEsim", b"IsEsim", b"IS_ESIM", b"eSimType", b"ESIM_TYPE", b"NOT eSIM"]:
    print(f"CP {n}: count={img.count(n)} first={hex(img.find(n)) if img.find(n)>=0 else None}")

# Find code that references log id 0x11d6 via the log helper pattern used in STATUS:
# MOVW r2,#id; MLA/CMP style — see 0x14fb366 pattern with r2=#0x106a
# Search MOVW r2,#0x11d6 or r1,#0x11d6 outside registration tables
print("\n=== MOVW #0x11d6 not in 0x1160xxx table ===")
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x11D6 and not (0x1160000 <= o <= 0x1162000):
        print(f"  {hex(o)} r{r[1]}")
        dump(o - 0x80, o + 0x20, f"real_11d6@{hex(o)}")
    o += 2

print("\n=== MOVW #0x11ce not in table ===")
o = 0x1000000
while o < 0x3C00000 - 4:
    r = movw(o)
    if r and r[0] == 0x11CE and not (0x1160000 <= o <= 0x1162000):
        print(f"  {hex(o)} r{r[1]}")
        dump(o - 0x80, o + 0x20, f"real_11ce@{hex(o)}")
    o += 2

# SupportedRatMap: find InitRapMap / writers
j = img.find(b"No CDMA in InitRapMap")
print(f"\nNoCDMA InitRapMap @{hex(j)}")
# Find "SupportedRatMap(%d)" usage via similar - search for unique substring as lit
# Look for STR to a global after loading a rat mask in QM_MM_STOP
j = img.find(b"@QM_MM_STOP_REQ_Handler: No CDMA in SupportedRatMap")
# The handler tests a bit - find via MOVW of low 16 of string if used as fmt in log
# Search code near QM_MM for TST/AND with CDMA bit values: 0x10, 0x20, 0x40, 0x100 etc.
# Better: find string "SupportedRatMap(0x%X)" and see if there's a writable global

# Search for property that sets rat map
for n in [
    b"ril.supported_rats",
    b"vendor.ril.rat",
    b"persist.vendor.radio.rat",
    b"ro.telephony.default_network",
    b"SupportedRat",
]:
    print(f"CP prop-like {n}: {img.find(n)}")

# stream BuildSetPreferredNetworkType - what msgid?
# Scan ELF symbols around the string for the function - look for immediate 0x070x in .text
# Prefer: strings "SetPreferredNetworkType" in request builder table
idx = stream.find(b"BuildSetPreferredNetworkType")
print(f"\nstream BuildSetPreferredNetworkType nearby ids:")
# In Samsung SIT, preferred network is often 0x0702 or similar - search stream for that name's xref
# Extract C++ name and find GOT - simpler: search stream ascii for "0x070" near Network
for n in [
    b"SIT_SET_PREFERRED_NETWORK",
    b"SIT_SET_DUAL_NETWORK",
    b"SIT_SET_CDMA",
    b"PREFERRED_NETWORK_TYPE",
]:
    print(f"  {n}: stream={stream.find(n)} img={img.find(n)}")

# Conclusion helper: does any proven SIT write SupportedRatMap?
# Search CP for store sites labeled by "SupportedRatMap" log after write
j = img.find(b"SupportedRatMap(%d)")
print(f"SupportedRatMap(%d) @{hex(j) if j>=0 else None}")
j2 = img.find(b"InitRapMap")
print(f"InitRapMap @{hex(j2)}")
# surrounding strings
for base in [j, j2]:
    if base is None or base < 0:
        continue
    chunk = img[base : base + 200]
    cur = bytearray()
    for b in chunk:
        if 32 <= b < 127:
            cur.append(b)
        else:
            if len(cur) >= 8:
                print(" ", cur.decode())
            cur = bytearray()
