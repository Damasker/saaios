#!/usr/bin/env python3
"""Decode ProtocolMisc NvRead/NvWrite builders + ids (RO)."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
stream = (DIAG / "sit-stream.so").read_bytes()
ril = (DIAG / "libsitril.so").read_bytes()
base = (DIAG / "sit-base.so").read_bytes()


def parse_dynsym(data: bytes) -> dict[str, int]:
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
    syms: dict[str, int] = {}
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
            name = strtab[st_name:].split(b"\0", 1)[0].decode("ascii", "replace")
            if name and st_value:
                syms[name] = st_value
    return syms


def va_to_off(data: bytes, addr: int) -> int | None:
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


def dump(data: bytes, addr: int, n: int = 0x140) -> None:
    off = va_to_off(data, addr)
    if off is None:
        print("  no map")
        return
    chunk = data[off : off + n]
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0x7F800000) == 0x52800000:
            imm16 = (insn >> 5) & 0xFFFF
            rd = insn & 0x1F
            hw = (insn >> 21) & 0x3
            val = imm16 << (hw * 16)
            print(f"  +{i:03x}: MOVZ W{rd},#{hex(val)}")
        elif (insn & 0xFFC00000) == 0x39000000:
            imm = (insn >> 10) & 0xFFF
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STRB W{rt},[X{rn},#{imm}]")
        elif (insn & 0xFFC00000) == 0x79000000:
            imm = ((insn >> 10) & 0xFFF) << 1
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STRH W{rt},[X{rn},#{imm}]")
        elif (insn & 0xFFC00000) == 0xB9000000:
            imm = ((insn >> 10) & 0xFFF) << 2
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STR W{rt},[X{rn},#{imm}]")
        elif (insn & 0xFC000000) == 0x94000000:
            imm = insn & 0x3FFFFFF
            if imm >= 0x2000000:
                imm -= 0x4000000
            target = addr + i + (imm << 2)
            print(f"  +{i:03x}: BL {hex(target)}")


ss = parse_dynsym(stream)
rs = parse_dynsym(ril)

print("=== NV-related symbols stream ===")
for k, v in sorted(ss.items()):
    if "Nv" in k or "NV" in k:
        print(hex(v), k[:160])

print("=== NV-related symbols ril ===")
for k, v in sorted(rs.items()):
    if "Nv" in k or "NV" in k:
        print(hex(v), k[:160])

print("=== string Nv across blobs ===")
for blob, name in ((stream, "stream"), (ril, "ril"), (base, "base")):
    for needle in (
        b"NvRead",
        b"NvWrite",
        b"NV_READ",
        b"NV_WRITE",
        b"MiscNv",
        b"BuildNv",
        b"ReadItem",
        b"WriteItem",
        b"nv_item",
        b"NvItem",
        b"SIT_MISC_NV",
        b"NvAccess",
        b"DoNv",
        b"nvread",
        b"nvwrite",
    ):
        i = 0
        c = 0
        while c < 12:
            j = blob.find(needle, i)
            if j < 0:
                break
            end = blob.find(b"\0", j)
            print(name, hex(j), blob[j:end][:160])
            i = j + 1
            c += 1

print("\n=== dump Build*Nv* ===")
for k, v in sorted(ss.items()):
    if "Nv" in k and ("Build" in k or "Builder" in k):
        print("BUILD", k, hex(v))
        dump(stream, v, 0x160)

print("\n=== dump ProtocolMiscNv* ===")
for k, v in sorted(ss.items()):
    if "MiscNv" in k:
        print("ADAPT", k, hex(v))
        dump(stream, v, 0x100)

print("\n=== ril DoNv / Nv handlers ===")
for k, v in sorted(rs.items()):
    if "Nv" in k or "NV" in k:
        if "Do" in k or "On" in k or "Handler" in k or "Request" in k:
            print(hex(v), k[:160])
