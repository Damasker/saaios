#!/usr/bin/env python3
"""Decode the RAN jump table inside TD1A BuildStartNetworkScan and the
band-convert tables it consults. Evidence only."""
import struct
from pathlib import Path

SO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so")
data = SO.read_bytes()

# ELF section map
e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]
sections = []
for i in range(e_shnum):
    o = e_shoff + i * e_shentsize
    sections.append({
        "name": struct.unpack_from("<I", data, o)[0],
        "addr": struct.unpack_from("<Q", data, o + 16)[0],
        "off": struct.unpack_from("<Q", data, o + 24)[0],
        "size": struct.unpack_from("<Q", data, o + 32)[0],
    })
shstr = sections[e_shstrndx]

def sname(n):
    o = shstr["off"] + n
    return data[o:data.find(b"\x00", o)].decode()

for s in sections:
    s["sname"] = sname(s["name"])

def va_off(va):
    for s in sections:
        if s["addr"] <= va < s["addr"] + max(s["size"], 1):
            return s["off"] + (va - s["addr"]), s["sname"]
    raise SystemExit(hex(va))

def u32(va):
    off, _ = va_off(va)
    return struct.unpack_from("<I", data, off)[0]

def u8(va):
    off, _ = va_off(va)
    return data[off]

# From the prologue constants (page 0xd8000):
# x15 = 0xd8000+2908, x16=+2941, x17=+2937, x0=+2933, x1=+2929,
# x2=+2925, x3=+2921, x6=+2917, x7=+2913
base_page = 0xD8000
ptrs = {
    "x15": base_page + 2908,
    "x16": base_page + 2941,
    "x17": base_page + 2937,
    "x0": base_page + 2933,
    "x1": base_page + 2929,
    "x2": base_page + 2925,
    "x3": base_page + 2921,
    "x6": base_page + 2917,
    "x7": base_page + 2913,
}
print("rodata pointers:")
for k, va in ptrs.items():
    off, sec = va_off(va)
    blob = data[off:off + 16]
    print(f"  {k} va=0x{va:x} {sec} bytes={blob.hex()}")

# RAN dispatch: adr x19, 0x2378b8; ldrb w24, [x15, w22, uxtw]; add x19,x19,w24,lsl#2; br x19
# w22 = RAN-1, RAN in 1..5 so index 0..4
table = ptrs["x15"]
anchor = 0x2378B8
print("\nRAN jump (index = RAN-1):")
for i in range(6):
    b = u8(table + i)
    tgt = anchor + (b << 2)
    ins = u32(tgt)
    print(f"  idx {i} byte=0x{b:02x} target=0x{tgt:x} first_insn=0x{ins:08x}")

# Also dump 32 bytes at each table pointer so band maps are visible.
print("\nnearby rodata window 0x{:x}:".format(ptrs["x7"]))
off, _ = va_off(ptrs["x7"])
window = data[off:off + 64]
print(window.hex())
