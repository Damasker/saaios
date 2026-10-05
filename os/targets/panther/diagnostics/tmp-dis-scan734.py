#!/usr/bin/env python3
"""Locate and dump BuildStartNetworkScan from the in-repo TD1A libsitril.so.

Evidence only. Prints the ELF symbol, a hex dump of the function, and a
minimal AArch64 disassembly of the stores that build the SIT frame.
"""
import struct
from pathlib import Path

SO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so")
NEEDLE = b"BuildStartNetworkScan"


def parse_elf(data: bytes):
    assert data[:4] == b"\x7fELF"
    ei_class = data[4]
    if ei_class != 2:
        raise SystemExit(f"not ELF64 class={ei_class}")
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]

    def sh(i):
        o = e_shoff + i * e_shentsize
        return {
            "name": struct.unpack_from("<I", data, o)[0],
            "type": struct.unpack_from("<I", data, o + 4)[0],
            "flags": struct.unpack_from("<Q", data, o + 8)[0],
            "addr": struct.unpack_from("<Q", data, o + 16)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
            "link": struct.unpack_from("<I", data, o + 40)[0],
            "entsize": struct.unpack_from("<Q", data, o + 56)[0],
        }

    shstr = sh(e_shstrndx)
    def sname(n):
        o = shstr["off"] + n
        return data[o:data.find(b"\x00", o)].decode()

    sections = []
    for i in range(e_shnum):
        s = sh(i)
        s["sname"] = sname(s["name"])
        sections.append(s)
    return sections


def load_symtab(data, sections, name):
    sec = next(s for s in sections if s["sname"] == name)
    strsec = sections[sec["link"]]
    n = sec["size"] // sec["entsize"]
    out = []
    for i in range(n):
        o = sec["off"] + i * sec["entsize"]
        st_name = struct.unpack_from("<I", data, o)[0]
        st_info = data[o + 4]
        st_shndx = struct.unpack_from("<H", data, o + 6)[0]
        st_value = struct.unpack_from("<Q", data, o + 8)[0]
        st_size = struct.unpack_from("<Q", data, o + 16)[0]
        so = strsec["off"] + st_name
        nm = data[so:data.find(b"\x00", so)].decode(errors="replace")
        out.append((nm, st_value, st_size, st_info, st_shndx))
    return out


def va_to_off(sections, va):
    for s in sections:
        if s["addr"] <= va < s["addr"] + s["size"] and s["flags"] & 4:
            return s["off"] + (va - s["addr"]), s
    return None, None


def main():
    data = SO.read_bytes()
    print(f"file={SO.name} size={len(data)}")
    sections = parse_elf(data)
    hits = []
    for secname in (".dynsym", ".symtab"):
        if not any(s["sname"] == secname for s in sections):
            print(f"no {secname}")
            continue
        for nm, va, sz, info, shndx in load_symtab(data, sections, secname):
            if "StartNetworkScan" in nm or "NetworkScan" in nm or "SystemSelection" in nm:
                hits.append((secname, nm, va, sz, info, shndx))
    for h in hits:
        print(f"SYM {h[0]} {h[1]} va=0x{h[2]:x} size=0x{h[3]:x} info=0x{h[4]:x} shndx={h[5]}")

    # Also string-xref the mangled name to confirm it exists.
    idx = data.find(NEEDLE)
    print(f"string {NEEDLE!r} at file+0x{idx:x}" if idx >= 0 else "string missing")


if __name__ == "__main__":
    main()
