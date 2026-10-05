#!/usr/bin/env python3
"""Clarify NvRead/NvWrite SIT layout + item-id enums; hunt TCS/CDMA item ids."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
FW = DIAG / "fw"
stream = (DIAG / "sit-stream.so").read_bytes()
ril = (DIAG / "libsitril.so").read_bytes()


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


def dump_window(data: bytes, addr: int, n: int) -> None:
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
        elif (insn & 0x7F800000) == 0x72A00000:  # MOVK W hw1?
            imm16 = (insn >> 5) & 0xFFFF
            rd = insn & 0x1F
            hw = (insn >> 21) & 0x3
            print(f"  +{i:03x}: MOVK W{rd},#{hex(imm16)},LSL#{hw*16}")
        elif (insn & 0xFFC00000) == 0x39000000:
            imm = (insn >> 10) & 0xFFF
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STRB W{rt},[X{rn},#{imm}]")
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
        elif (insn & 0xFC000000) == 0x14000000:
            imm = insn & 0x3FFFFFF
            if imm >= 0x2000000:
                imm -= 0x4000000
            target = addr + i + (imm << 2)
            print(f"  +{i:03x}: B {hex(target)}")
        elif insn == 0xD65F03C0:
            print(f"  +{i:03x}: RET")
            break


ss = parse_dynsym(stream)
rs = parse_dynsym(ril)

print("=== BuildNvReadItem tiny window ===")
dump_window(stream, ss["_ZN19ProtocolMiscBuilder15BuildNvReadItemEi"], 0x40)
print("=== BuildNvWriteItem tiny window ===")
dump_window(stream, ss["_ZN19ProtocolMiscBuilder16BuildNvWriteItemEiPKc"], 0x100)

# Follow likely shared InitRequestHeader helpers: look at first BL targets from write
print("\n=== DoNvReadItem / DoNvWriteItem / ProcessNvReadItem dumps ===")
for name in (
    "_ZN11MiscService12DoNvReadItemEP7Message",
    "_ZN11MiscService13DoNvWriteItemEP7Message",
    "_ZN11MiscService17ProcessNvReadItemEiPKc",
    "_ZN17RilRequestCreator16CreateNvReadItemEiPvPcj",
    "_ZN17RilRequestCreator17CreateNvWriteItemEiPvPcj",
):
    if name in rs:
        print(name, hex(rs[name]))
        dump_window(ril, rs[name], 0x180)

print("\n=== strings near Nv item enums / RIL_REQUEST_NV ===")
for needle in (
    b"RIL_REQUEST_NV_READ_ITEM",
    b"RIL_REQUEST_NV_WRITE_ITEM",
    b"NV_READ_ITEM",
    b"NV_WRITE_ITEM",
    b"CDMA_SUPPORT",
    b"TCS_CDMA",
    b"SupportedRat",
    b"NV_CDMA",
    b"itemId",
    b"nvItemId",
    b"NvItemId",
    b"SIT_MISC_NV_READ",
    b"SIT_MISC_NV_WRITE",
    b"0x090c",
    b"0x090d",
    b"0x90c",
    b"0x90d",
):
    i = 0
    c = 0
    while c < 8:
        j = ril.find(needle, i)
        if j < 0:
            break
        end = ril.find(b"\0", j)
        print("ril", hex(j), ril[j:end][:160])
        i = j + 1
        c += 1
    i = 0
    c = 0
    while c < 8:
        j = stream.find(needle, i)
        if j < 0:
            break
        end = stream.find(b"\0", j)
        print("stream", hex(j), stream[j:end][:160])
        i = j + 1
        c += 1

# Android RIL NV item ids are often small enums (OEM-specific).
# Search for MOVZ immediates near CreateNvReadItem that look like item tables.
print("\n=== immediates near CreateNvReadItem ===")
addr = rs.get("_ZN17RilRequestCreator16CreateNvReadItemEiPvPcj")
if addr:
    off = va_to_off(ril, addr)
    chunk = ril[off : off + 0x200]
    vals = []
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0x7F800000) == 0x52800000:
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            vals.append(imm16 << (hw * 16))
    print([hex(v) for v in vals[:40]])

print("\n=== MAIN image: TCS_CDMA / NV item id clues ===")
img = (FW / "saaios-probe-b-modem.bin").read_bytes()
for needle in (
    b"TCS_CDMA_SUPPORT",
    b"DS_TCS_GV_CDMA_SUPPORT",
    b"@[TCS] GV updated from reg",
    b"NV_READ",
    b"NvWrite",
    b"SIT_MISC_NV",
    b"CDMA_SUPPORT",
):
    j = img.find(needle)
    print(needle, hex(j) if j >= 0 else None)
    if j is not None and j >= 0:
        # nearby printable
        window = img[max(0, j - 64) : j + 128]
        strs = []
        cur = b""
        for b in window:
            if 32 <= b < 127:
                cur += bytes([b])
            else:
                if len(cur) >= 6:
                    strs.append(cur.decode())
                cur = b""
        print("  nearby:", strs[:12])

# GV id 0x127 previously documented — search for #0x127 near TCS string refs is hard;
# report GV id string contexts
j = img.find(b"DS_TCS_GV_CDMA_SUPPORT")
print("DS_TCS_GV_CDMA_SUPPORT @", hex(j) if j >= 0 else None)
