#!/usr/bin/env python3
"""Disassemble GetScanStatus and the status compare in OnIndication."""
import struct
from pathlib import Path

SO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so")
data = SO.read_bytes()
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
        "link": struct.unpack_from("<I", data, o + 40)[0],
        "entsize": struct.unpack_from("<Q", data, o + 56)[0],
    })
shstr = sections[e_shstrndx]

def sname(n):
    o = shstr["off"] + n
    return data[o:data.find(b"\x00", o)].decode()

for s in sections:
    s["sname"] = sname(s["name"])

dyn = next(s for s in sections if s["sname"] == ".dynsym")
dstr = sections[dyn["link"]]
target = None
for i in range(dyn["size"] // dyn["entsize"]):
    o = dyn["off"] + i * dyn["entsize"]
    st_name = struct.unpack_from("<I", data, o)[0]
    va = struct.unpack_from("<Q", data, o + 8)[0]
    sz = struct.unpack_from("<Q", data, o + 16)[0]
    so = dstr["off"] + st_name
    nm = data[so:data.find(b"\x00", so)].decode(errors="replace")
    if nm.endswith("GetScanStatusEv") or nm.endswith("OnIndicationEP7Message") and "NetworkScan" in nm:
        print(f"{nm} va=0x{va:x} size=0x{sz:x}")
        if nm.endswith("GetScanStatusEv"):
            target = (va, sz)

def va_off(va):
    for s in sections:
        if s["addr"] <= va < s["addr"] + max(s["size"], 1):
            return s["off"] + (va - s["addr"])
    raise SystemExit(hex(va))

if target:
    va, sz = target
    off = va_off(va)
    blob = data[off:off + sz]
    print("GetScanStatus bytes:")
    for i in range(0, len(blob), 4):
        w = struct.unpack_from("<I", blob, i)[0]
        print(f"  0x{va+i:x}: {w:08x}")
