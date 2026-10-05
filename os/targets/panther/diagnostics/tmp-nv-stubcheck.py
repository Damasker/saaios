#!/usr/bin/env python3
"""Verify whether BuildNv* are stubs; find 0x90c/0x90d SIT id sites; map adapter."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
stream = (DIAG / "sit-stream.so").read_bytes()
ril = (DIAG / "libsitril.so").read_bytes()


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
    syms = {}
    sizes = {}
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
rs, rsz = parse_dynsym(ril)

for name in (
    "_ZN19ProtocolMiscBuilder15BuildNvReadItemEi",
    "_ZN19ProtocolMiscBuilder16BuildNvWriteItemEiPKc",
):
    addr = ss[name]
    off = va_to_off(stream, addr)
    print(name, "va", hex(addr), "size", hex(ssz.get(name, 0)), "bytes", stream[off : off + 16].hex())

# Find MOVZ W*, #0x90c / #0x90d in stream (AArch64 MOVZ encoding)
# MOVZ Wd, #imm16, LSL #0 : 0x52800000 | (imm16<<5) | rd
# 0x90c = 2316


def find_movz(data: bytes, imm: int):
    hits = []
    enc_base = 0x52800000 | (imm << 5)
    for rd in range(32):
        enc = struct.pack("<I", enc_base | rd)
        i = 0
        while True:
            j = data.find(enc, i)
            if j < 0:
                break
            hits.append((j, rd))
            i = j + 1
    return hits


print("\n=== MOVZ #0x90c sites in stream ===")
for off, rd in find_movz(stream, 0x90C):
    print(hex(off), "W" + str(rd), "context", stream[off - 8 : off + 24].hex())

print("\n=== MOVZ #0x90d sites in stream ===")
for off, rd in find_movz(stream, 0x90D):
    print(hex(off), "W" + str(rd), "context", stream[off - 8 : off + 24].hex())

# Also halfword little-endian 0x0c09 / 0x0d09 as id fields in tables
print("\n=== raw u16 0x090c / 0x090d occurrences (sample) ===")
for needle, label in ((b"\x0c\x09", "0x090c"), (b"\x0d\x09", "0x090d")):
    i = 0
    c = 0
    while c < 20:
        j = stream.find(needle, i)
        if j < 0:
            break
        # skip if likely ASCII
        print(label, hex(j), stream[j - 4 : j + 8].hex())
        i = j + 1
        c += 1

# Relocs / got for BuildNv — check if callers exist in ril
print("\n=== libsitril xrefs to Nv builders (string/plt names) ===")
for needle in (
    b"BuildNvReadItem",
    b"BuildNvWriteItem",
    b"DoNvReadItem",
    b"DoNvWriteItem",
    b"NvReadItem",
    b"NvWriteItem",
    b"NV_READ_ITEM",
    b"NV_WRITE_ITEM",
):
    i = 0
    c = 0
    while c < 10:
        j = ril.find(needle, i)
        if j < 0:
            break
        end = ril.find(b"\0", j)
        print(hex(j), ril[j:end][:120])
        i = j + 1
        c += 1

# Dump DoNvReadItem full-ish with more insn classes
print("\n=== DoNvReadItem detailed ===")
addr = rs["_ZN11MiscService12DoNvReadItemEP7Message"]
off = va_to_off(ril, addr)
chunk = ril[off : off + 0xC0]
print("size", hex(rsz.get("_ZN11MiscService12DoNvReadItemEP7Message", 0)))
print("hex", chunk[:64].hex())
for i in range(0, len(chunk) - 3, 4):
    insn = struct.unpack_from("<I", chunk, i)[0]
    if (insn & 0x7F800000) == 0x52800000:
        imm16 = (insn >> 5) & 0xFFFF
        rd = insn & 0x1F
        print(f"  +{i:03x}: MOVZ W{rd},#{hex(imm16)}")
    elif (insn & 0xFC000000) == 0x94000000:
        imm = insn & 0x3FFFFFF
        if imm >= 0x2000000:
            imm -= 0x4000000
        print(f"  +{i:03x}: BL {hex(addr+i+(imm<<2))}")
    elif (insn & 0xFF00001F) == 0x54000000:
        print(f"  +{i:03x}: B.cond")
    elif insn == 0xD65F03C0:
        print(f"  +{i:03x}: RET")
        break

# Check Android RIL request numbers for NV in ril
print("\n=== RIL_REQUEST numbers near CreateNv* (search MOVZ #0x31 / #0x32 classic) ===")
# Classic AOSP: RIL_REQUEST_NV_READ_ITEM = 101, NV_WRITE = 102
for imm in (101, 102, 0x65, 0x66, 0x90C, 0x90D, 0x31, 0x32):
    hits = find_movz(ril, imm)
    if hits:
        print(hex(imm), "count", len(hits), "first", hex(hits[0][0]), "W"+str(hits[0][1]))

# ProtocolMiscBuilder vtable / nearby builders with real bodies for context
print("\n=== ProtocolMiscBuilder Build* with non-trivial size near Nv ===")
for k, v in sorted(ss.items()):
    if "ProtocolMiscBuilder" in k and "Build" in k:
        sz = ssz.get(k, 0)
        if sz and sz > 8:
            print(hex(v), "sz", hex(sz), k[30:90])
        elif "Nv" in k:
            print(hex(v), "sz", hex(sz), "STUB?", k[30:90])
