#!/usr/bin/env python3
"""Decode carrier/SGC/TCS-adjacent builders; confirm NV stubs; hunt real NV ids."""
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


def decode_builder(data: bytes, addr: int, size: int):
    off = va_to_off(data, addr)
    chunk = data[off : off + max(size, 0x40)]
    ids, lens = [], []
    print("  bytes", chunk[:16].hex())
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0x7F800000) == 0x52800000:
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            val = imm16 << (hw * 16)
            rd = insn & 0x1F
            if 0x200 <= val <= 0x9FF or 0x4600 <= val <= 0x46FF:
                ids.append(val)
                print(f"  +{i:03x}: MOVZ W{rd},#{hex(val)}  (SIT id cand)")
            elif 8 <= val <= 400:
                lens.append(val)
                print(f"  +{i:03x}: MOVZ W{rd},#{hex(val)}  (len cand)")
            else:
                print(f"  +{i:03x}: MOVZ W{rd},#{hex(val)}")
        elif insn == 0xD65F03C0:
            print(f"  +{i:03x}: RET")
            break
    return ids, lens


ss, ssz = parse_dynsym(stream)
rs, rsz = parse_dynsym(ril)

targets = [
    "BuildNvReadItem",
    "BuildNvWriteItem",
    "BuildModemActivityInfo",
    "BuildSetOpenCarierInfo",
    "BuildGetCdmaSubscription",
    "BuildSetSGCValue",
    "BuildSetCpCarrierConfig",
    "BuildSetOemSetSvn",
    "BuildCdmaSubscription",
    "BuildSetCarrierInfoImsiEncryption",
    "BuildRadioConfigReset",
    "BuildSetElevatorSensor",
]

print("=== selected builders ===")
for k, v in sorted(ss.items()):
    if any(t in k for t in targets) and ("ProtocolMiscBuilder" in k or "ProtocolNetworkBuilder" in k):
        print("\n", k[20:120] if len(k) > 20 else k, hex(v), "sz", hex(ssz.get(k, 0)))
        decode_builder(stream, v, ssz.get(k, 0x80))

# NvResetConfigHandler — what SIT does it send?
print("\n=== NvResetConfigHandler OnRequest ===")
addr = rs["_ZN20NvResetConfigHandler9OnRequestEP7Message"]
decode_builder(ril, addr, rsz.get("_ZN20NvResetConfigHandler9OnRequestEP7Message", 0x100))

# Search for call to BuildNv* from ril via PLT / string "NvReadItem" dispatch table
print("\n=== MiscService request id table clues (strings) ===")
for needle in (
    b"DoNvReadItem",
    b"DoNvWriteItem",
    b"NvResetConfig",
    b"RESET_CONFIG",
    b"RIL_REQUEST_NV",
    b"REQUEST_NV",
    b"SGCValue",
    b"CpCarrierConfig",
    b"OpenCarierInfo",
    b"OpenCarrierInfo",
    b"SetSGC",
    b"CDMA_SUBSCRIPTION",
):
    i = 0
    c = 0
    while c < 6:
        j = ril.find(needle, i)
        if j < 0:
            break
        end = ril.find(b"\0", j)
        print(hex(j), ril[j:end][:140])
        i = j + 1
        c += 1

# Confirm: are there ANY non-stub NV builders in sit-base?
print("\n=== sit-base Nv symbols ===")
base = (DIAG / "sit-base.so").read_bytes()
bs, bsz = parse_dynsym(base)
for k, v in sorted(bs.items()):
    if "Nv" in k or "NV" in k:
        print(hex(v), "sz", hex(bsz.get(k, 0)), k[:120])

# Adapter GetValue — does response path exist for NV read even if build is stub?
print("\n=== ProtocolMiscNvReadItemAdapter GetValue size ===")
name = "_ZN29ProtocolMiscNvReadItemAdapter8GetValueEv"
print(hex(ss[name]), "sz", hex(ssz.get(name, 0)))
decode_builder(stream, ss[name], ssz.get(name, 0x40))

# Verdict helpers: enumerate all ProtocolMiscBuilder with size==8 (stubs)
print("\n=== ALL ProtocolMiscBuilder size==8 stubs ===")
for k, v in sorted(ss.items()):
    if "ProtocolMiscBuilder" in k and ssz.get(k, 0) == 8:
        print(hex(v), k[30:100])
