#!/usr/bin/env python3
"""Decode BuildSetSGCValue; check A vs B for CDMA in RatMap strings only."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
stream = (DIAG / "sit-stream.so").read_bytes()
FW = DIAG / "fw"


def parse_dynsym(data: bytes):
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]

    def sh(i: int):
        o = e_shoff + i * e_shentsize
        return {
            "name": struct.unpack_from("<I", data, o)[0],
            "addr": struct.unpack_from("<Q", data, o + 16)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
            "link": struct.unpack_from("<I", data, o + 40)[0],
            "entsize": struct.unpack_from("<Q", data, o + 56)[0],
        }

    names = data[sh(e_shstrndx)["off"] : sh(e_shstrndx)["off"] + sh(e_shstrndx)["size"]]
    syms, sizes = {}, {}
    for i in range(e_shnum):
        s = sh(i)
        nm = names[s["name"] :].split(b"\0", 1)[0]
        if nm not in (b".dynsym", b".symtab"):
            continue
        link = sh(s["link"])
        strtab = data[link["off"] : link["off"] + link["size"]]
        ents = s["entsize"] or 24
        for j in range(0, s["size"], ents):
            o = s["off"] + j
            st_name = struct.unpack_from("<I", data, o)[0]
            st_value = struct.unpack_from("<Q", data, o + 8)[0]
            st_size = struct.unpack_from("<Q", data, o + 16)[0]
            name = strtab[st_name:].split(b"\0", 1)[0].decode("ascii", "replace")
            if name and st_value:
                syms[name] = st_value
                sizes[name] = st_size
    return syms, sizes


def va_to_off(data: bytes, addr: int):
    e_phoff = struct.unpack_from("<Q", data, 0x20)[0]
    e_phentsize = struct.unpack_from("<H", data, 0x36)[0]
    e_phnum = struct.unpack_from("<H", data, 0x38)[0]
    for i in range(e_phnum):
        o = e_phoff + i * e_phentsize
        if struct.unpack_from("<I", data, o)[0] != 1:
            continue
        p_offset = struct.unpack_from("<Q", data, o + 8)[0]
        p_vaddr = struct.unpack_from("<Q", data, o + 16)[0]
        p_filesz = struct.unpack_from("<Q", data, o + 32)[0]
        if p_vaddr <= addr < p_vaddr + p_filesz:
            return p_offset + (addr - p_vaddr)
    return None


ss, ssz = parse_dynsym(stream)
for k, v in sorted(ss.items()):
    if "SGC" in k or "Sgc" in k:
        print(k, hex(v), "sz", hex(ssz.get(k, 0)))
        off = va_to_off(stream, v)
        chunk = stream[off : off + ssz.get(k, 0x80)]
        for i in range(0, len(chunk) - 3, 4):
            insn = struct.unpack_from("<I", chunk, i)[0]
            if (insn & 0x7F800000) == 0x52800000:
                imm16 = (insn >> 5) & 0xFFFF
                hw = (insn >> 21) & 0x3
                val = imm16 << (hw * 16)
                if 0x200 <= val <= 0x9FF or 8 <= val <= 400:
                    print(f"  +{i:03x}: #{hex(val)}")

print("\n=== A/B CDMA RatMap string presence ===")
for name in ("saaios-probe-a-modem.bin", "saaios-probe-b-modem.bin"):
    img = (FW / name).read_bytes()
    checks = {
        "No CDMA SupportedRatMap": img.find(b"No CDMA in SupportedRatMap") >= 0,
        "No CDMA InitRapMap": img.find(b"No CDMA in InitRapMap") >= 0,
        "TCS_CDMA_SUPPORT": img.find(b"TCS_CDMA_SUPPORT") >= 0,
        "CDMA in SupportedRatMap positive?": img.find(b"CDMA in SupportedRatMap") >= 0,
        "version A": b"g5300q-251202" in img,
        "version B": b"g5300q-260317" in img,
    }
    print(name, checks)
