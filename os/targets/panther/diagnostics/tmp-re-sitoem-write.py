#!/usr/bin/env python3
"""RE SitOemHandler / ModemData write path in carved oem_ipc0 ELF."""
from __future__ import annotations

import struct
from pathlib import Path

blob = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
).read_bytes()
print(f"size={len(blob)}")

# Dump all printable C++ sym-like and log strings
strings = []
i = 0
while i < len(blob):
    if 32 <= blob[i] < 127:
        j = i
        while j < len(blob) and 32 <= blob[j] < 127:
            j += 1
        if j - i >= 6:
            s = blob[i:j].decode()
            strings.append((i, s))
        i = j + 1
    else:
        i += 1

keys = (
    "SitOem",
    "ModemData",
    "oem_ipc",
    "protobuf",
    "Serial",
    "sendModem",
    "writeModem",
    "synchronous",
    "Request",
    "SIM",
    "Init",
    "token",
    "Token",
    "header",
    "Header",
    "length",
    "Length",
    "encode",
    "Encode",
    "ipc",
    "IPC",
    "open",
    "channel",
)
print("=== interesting strings ===")
for off, s in strings:
    if any(k in s for k in keys):
        print(f"  {hex(off)}: {s[:140]}")

# Parse ELF to get .dynstr / .dynsym names
def u16(o):
    return struct.unpack_from("<H", blob, o)[0]


def u32(o):
    return struct.unpack_from("<I", blob, o)[0]


def u64(o):
    return struct.unpack_from("<Q", blob, o)[0]


e_shoff = u64(40)
e_shentsize = u16(58)
e_shnum = u16(60)
e_shstrndx = u16(62)
print(f"\nELF shoff={e_shoff} shnum={e_shnum} shstrndx={e_shstrndx}")

# section header string table
shstr_off = u64(e_shoff + e_shstrndx * e_shentsize + 24)
shstr_size = u64(e_shoff + e_shstrndx * e_shentsize + 32)
shstr = blob[shstr_off : shstr_off + shstr_size]


def secname(name_off):
    end = shstr.find(b"\x00", name_off)
    return shstr[name_off:end].decode()


sections = {}
for i in range(e_shnum):
    off = e_shoff + i * e_shentsize
    name = secname(u32(off))
    sh_type = u32(off + 4)
    sh_addr = u64(off + 16)
    sh_offset = u64(off + 24)
    sh_size = u64(off + 32)
    sections[name] = dict(type=sh_type, addr=sh_addr, offset=sh_offset, size=sh_size)
    if name in (".dynsym", ".dynstr", ".symtab", ".strtab", ".rodata", ".text", ".data"):
        print(f"  {name}: off={hex(sh_offset)} size={hex(sh_size)} addr={hex(sh_addr)}")

# dynsym exports
if ".dynsym" in sections and ".dynstr" in sections:
    dynsym = sections[".dynsym"]
    dynstr = blob[
        sections[".dynstr"]["offset"] : sections[".dynstr"]["offset"] + sections[".dynstr"]["size"]
    ]
    entsize = 24
    print("\n=== dynsym (filtered) ===")
    for i in range(dynsym["size"] // entsize):
        o = dynsym["offset"] + i * entsize
        st_name = u32(o)
        st_info = blob[o + 4]
        st_value = u64(o + 8)
        st_size = u64(o + 16)
        end = dynstr.find(b"\x00", st_name)
        name = dynstr[st_name:end].decode()
        if any(
            k in name
            for k in (
                "SitOem",
                "ModemData",
                "oem",
                "Oem",
                "protobuf",
                "Serial",
                "writeModem",
                "sendModem",
                "synchronous",
                "Transmitter",
                "Request",
            )
        ):
            print(f"  {hex(st_value)} size={st_size} {name}")

# Find xref: string /dev/oem_ipc0 VA = load_addr + offset
# For ET_DYN, use section addr
text = sections.get(".text")
rodata = sections.get(".rodata")
# find which section contains oem_ipc0 string
oem_off = blob.find(b"/dev/oem_ipc0")
print(f"\noem_ipc0 file_off={hex(oem_off)}")
for name, s in sections.items():
    if s["offset"] <= oem_off < s["offset"] + s["size"]:
        va = s["addr"] + (oem_off - s["offset"])
        print(f"  in {name} va={hex(va)}")
        # ADRP+ADD search for page
        page = va & ~0xFFF
        # ADRP: immlo in bits 30:29, immhi in 23:5
        hits = []
        for i in range(0, len(blob) - 8, 4):
            w = u32(i)
            if (w & 0x9F000000) == 0x90000000:  # ADRP
                immhi = (w >> 5) & 0x7FFFF
                immlo = (w >> 29) & 0x3
                imm = (immhi << 2) | immlo
                if imm & (1 << 20):
                    imm -= 1 << 21
                # need PC of instruction
                # file offset i -> VA: find section
                # approximate: if in .text
                if text and text["offset"] <= i < text["offset"] + text["size"]:
                    pc = text["addr"] + (i - text["offset"])
                    target_page = (pc & ~0xFFF) + (imm << 12)
                    if target_page == page:
                        hits.append((i, pc, w & 0x1F))
        print(f"  ADRP to page hits={len(hits)} {[hex(h[0]) for h in hits[:10]]}")
        for i, pc, rd in hits[:5]:
            # next few instr for ADD imm
            print(f"  --- @{hex(i)} pc={hex(pc)} rd=x{rd} ---")
            for j in range(i, i + 32, 4):
                w = u32(j)
                print(f"    {hex(j)}: {w:08x}")

# Look at sendModemRawData / writeModemData disasm via dynsym value
print("\n=== probe protobuf length prefix patterns near ModemData methods ===")
# Search for common patterns: STRH/STR of length then write
# MOVZ #0x0a (protobuf wire) etc — broad

# Extract more complete binary: maybe carved size too small and dynsym incomplete
# Check if file starts with valid ELF and has interpreter
print("\n=== ELF header ===")
print("type", u16(16), "machine", u16(18), "entry", hex(u64(24)))
print("DONE")
