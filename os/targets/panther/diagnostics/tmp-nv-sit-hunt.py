#!/usr/bin/env python3
"""Hunt signed SIT NV/OEM/TCS/CDMA builders in sit-stream / libsitril (RO)."""
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


def decode_id(data: bytes, addr: int, window: int = 0x180):
    off = va_to_off(data, addr)
    if off is None:
        return None, None, []
    chunk = data[off : off + window]
    ids, lens, movs = [], [], []
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0x7F800000) == 0x52800000:  # MOVZ W
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            val = imm16 << (hw * 16)
            movs.append(val)
            if (
                0x0200 <= val <= 0x02FF
                or 0x0400 <= val <= 0x09FF
                or 0x4600 <= val <= 0x46FF
                or 0x5000 <= val <= 0x5FFF
            ):
                ids.append(val)
            if 8 <= val <= 400:
                lens.append(val)
    return (
        ids[0] if ids else None,
        lens[0] if lens else None,
        movs[:24],
    )


ss = parse_dynsym(stream)
rs = parse_dynsym(ril)

keys = (
    "Nv",
    "NV",
    "Oem",
    "OEM",
    "Tcs",
    "TCS",
    "Efs",
    "EFS",
    "Factory",
    "Secure",
    "Registry",
    "Rat",
    "Cdma",
    "CDMA",
    "Item",
    "Eng",
    "Debug",
    "Write",
)

print("=== sit-stream symbols matching NV/OEM/TCS/CDMA/Factory/Secure/EFS/Item/Eng ===")
for k, v in sorted(ss.items(), key=lambda x: x[0]):
    if any(x.lower() in k.lower() for x in keys):
        if "Build" in k or "Protocol" in k or "Builder" in k:
            oid, olen, _ = decode_id(stream, v)
        else:
            oid, olen = None, None
        print(f"{hex(v)} id={oid and hex(oid)} len={olen} {k[:130]}")

print("\n=== libsitril symbols matching same ===")
for k, v in sorted(rs.items()):
    if any(x.lower() in k.lower() for x in keys):
        print(hex(v), k[:140])

print("\n=== string scan ===")
for blob, name in ((stream, "stream"), (ril, "ril"), (base, "base")):
    for needle in (
        b"BuildNv",
        b"NvWrite",
        b"WriteNv",
        b"SetNv",
        b"GetNv",
        b"OemHook",
        b"OEM_HOOK",
        b"FactoryMode",
        b"SecureNv",
        b"TCS_CDMA",
        b"SupportedRat",
        b"WriteItem",
        b"SetItem",
        b"GetItem",
        b"BuildOem",
        b"ProtocolOem",
        b"NvItem",
        b"NV_ITEM",
        b"efs_write",
        b"RfsWrite",
        b"SetEngMode",
        b"BuildSetEngMode",
        b"CDMA_SUPPORT",
        b"RatMap",
        b"nv_write",
        b"NvAccess",
        b"AccessNv",
    ):
        i = 0
        c = 0
        while c < 5:
            j = blob.find(needle, i)
            if j < 0:
                break
            end = blob.find(b"\0", j)
            print(name, hex(j), blob[j:end][:120])
            i = j + 1
            c += 1

print("\n=== Build*Cdma* / Rat* builders ===")
for k, v in sorted(ss.items()):
    if "Cdma" in k or "CDMA" in k or "cdma" in k or "Rat" in k:
        oid, olen, _ = decode_id(stream, v)
        print(f"{hex(v)} id={oid and hex(oid)} len={olen} {k[:130]}")

print("\n=== MiscDebug / EngMode builders ===")
for k, v in sorted(ss.items()):
    if "Eng" in k or "Debug" in k or "Factory" in k:
        oid, olen, movs = decode_id(stream, v)
        print(
            f"{hex(v)} id={oid and hex(oid)} len={olen} "
            f"movs={[hex(x) for x in movs[:10]]} {k[:120]}"
        )
