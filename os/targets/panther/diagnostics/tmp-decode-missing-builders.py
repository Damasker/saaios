#!/usr/bin/env python3
"""Decode never-sent stock builders: SetMobileDataState, DualNetwork, 3GPbCapa, VoiceOp SET."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
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

    shstr = sh(e_shstrndx)
    names = data[shstr["off"] : shstr["off"] + shstr["size"]]
    syms = {}
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


def disasm_movs(data: bytes, addr: int, n: int = 0x120) -> None:
    off = va_to_off(data, addr)
    assert off is not None
    chunk = data[off : off + n]
    print(f"--- disasm @{hex(addr)} file@{hex(off)} ---")
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        va = addr + i
        # MOVZ W
        if (insn & 0x7F800000) == 0x52800000:
            rd = insn & 0x1F
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            val = imm16 << (hw * 16)
            print(f"  {hex(va)} MOVZ w{rd},#{hex(val)} ({val})")
        elif (insn & 0x7F800000) == 0x12A00000:  # MOVN W rough? skip
            pass
        elif (insn & 0xFFC00000) == 0x91000000:  # ADD X
            pass
        elif (insn & 0xFFE0FC00) == 0x39000000:  # STRB
            rt = insn & 0x1F
            rn = (insn >> 5) & 0x1F
            imm12 = (insn >> 10) & 0xFFF
            print(f"  {hex(va)} STRB w{rt},[x{rn},#{imm12}]")
        elif (insn & 0xFFE0FC00) == 0x79000000:  # STRH
            rt = insn & 0x1F
            rn = (insn >> 5) & 0x1F
            imm12 = ((insn >> 10) & 0xFFF) << 1
            print(f"  {hex(va)} STRH w{rt},[x{rn},#{imm12}]")
        elif (insn & 0xFFE0FC00) == 0xB9000000:  # STR W
            rt = insn & 0x1F
            rn = (insn >> 5) & 0x1F
            imm12 = ((insn >> 10) & 0xFFF) << 2
            print(f"  {hex(va)} STR w{rt},[x{rn},#{imm12}]")
        elif (insn & 0xFC000000) == 0x94000000:
            imm26 = insn & 0x03FFFFFF
            if imm26 & 0x02000000:
                imm26 -= 0x04000000
            tgt = va + (imm26 << 2)
            print(f"  {hex(va)} BL {hex(tgt)}")
        elif insn == 0xD65F03C0:
            print(f"  {hex(va)} RET")
            break


ss = parse_dynsym(stream)
rs = parse_dynsym(ril)

targets = [
    "BuildSetMobileDataState",
    "BuildSetDualNetwork",
    "BuildSim3GPbCapa",
    "BuildSetVoiceOperation",
    "BuildGetPreferredCallCapability",
    "BuildRadioConfigReset",
    "BuildSetPreferredDataModem",
]
for needle in targets:
    for k, v in ss.items():
        if needle in k:
            print(f"\n=== {k[:100]} @ {hex(v)} ===")
            disasm_movs(stream, v, 0x100)

# Find libsitril callers that invoke SetMobileDataState / 3GPbCapa by string logs
print("\n=== libsitril log strings near MobileData / PbCapa / VoiceOperation ===")
for needle in (
    b"SetMobileDataState",
    b"MobileDataState",
    b"3GPbCapa",
    b"PbCapa",
    b"SetVoiceOperation",
    b"GetVoiceOperation",
    b"PreferredCallCapability",
    b"RadioConfigReset",
    b"PreferredDataModem",
    b"DualNetwork",
):
    idx = 0
    while True:
        j = ril.find(needle, idx)
        if j < 0:
            break
        ctx = ril[max(0, j - 40) : j + 80]
        printable = "".join(chr(b) if 32 <= b < 127 else "." for b in ctx)
        print(f"  ril@{hex(j)} ...{printable}...")
        idx = j + 1
        if idx > j + 1 and ril.find(needle, idx) == j:
            break
        # limit
        if idx - (ril.find(needle)) > 5000:
            break
        if idx > j + 200:
            # only first few
            hits = list(range(5))
            break
