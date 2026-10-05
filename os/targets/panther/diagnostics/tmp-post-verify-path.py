#!/usr/bin/env python3
"""Decode OnGetSimStatusDone / post-VerifyPin SIT solicit; find empty GETs we may never send."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"
)
ril = (DIAG / "libsitril.so").read_bytes()
stream = (DIAG / "sit-stream.so").read_bytes()


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


def bl_targets(data: bytes, func_va: int, size: int = 0x400) -> list[int]:
    """Collect BL immediate targets from AArch64 function."""
    off = va_to_off(data, func_va)
    if off is None:
        return []
    chunk = data[off : off + size]
    out = []
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0xFC000000) != 0x94000000:
            continue
        imm26 = insn & 0x03FFFFFF
        if imm26 & 0x02000000:
            imm26 -= 0x04000000
        target = func_va + i + (imm26 << 2)
        out.append(target)
    return out


def decode_id_len(data: bytes, addr: int, window: int = 0x100):
    off = va_to_off(data, addr)
    if off is None:
        return None, None
    chunk = data[off : off + window]
    ids, lens = [], []
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        if (insn & 0x7F800000) == 0x52800000:  # MOVZ W
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 0x3
            val = imm16 << (hw * 16)
            if 0x0200 <= val <= 0x02FF or 0x0600 <= val <= 0x09FF or 0x4600 <= val <= 0x46FF:
                ids.append(val)
            if 8 <= val <= 300:
                lens.append(val)
    idv = ids[0] if ids else None
    lenv = None
    for L in lens:
        if L in (12, 13, 14, 15, 16, 17, 18, 21, 24, 29, 30, 38, 48, 63, 71, 72):
            lenv = L
            break
    return idv, lenv


rsyms = parse_dynsym(ril)
ssyms = parse_dynsym(stream)

# Invert stream symbols for BL match
stream_by_va = {v: k for k, v in ssyms.items()}
ril_by_va = {v: k for k, v in rsyms.items()}


def nearest_sym(table: dict[int, str], va: int, window: int = 0x20) -> str | None:
    if va in table:
        return table[va]
    # PLT stubs etc — match exact only
    return None


focus = [
    "OnGetSimStatusDone",
    "CheckAndAutoVerifyPin",
    "DoVerifyPin",
    "DoAutoVerifyPin",
    "DoGetImsi",
    "OnVerifyPinDone",
    "DoGetSimStatus",
    "DoRadioPower",
    "OnRadioPowerDone",
    "OnRadioStateChanged",
    "SimService19OnRadioStateChanged",
]

print("=== BL targets from key libsitril handlers ===")
for needle in focus:
    matches = [(k, v) for k, v in rsyms.items() if needle in k]
    for k, v in sorted(matches, key=lambda x: x[1])[:3]:
        targets = bl_targets(ril, v, 0x500)
        print(f"\n# {k} @ {hex(v)} BLs={len(targets)}")
        # resolve against ril + note Build* if string nearby in stream via plt unknown
        named = []
        for t in targets:
            nk = ril_by_va.get(t)
            if nk:
                short = nk
                if len(short) > 100:
                    short = short[:100]
                named.append(short)
        # unique preserve order
        seen = set()
        for n in named:
            if n in seen:
                continue
            seen.add(n)
            interesting = any(
                x in n
                for x in (
                    "Build",
                    "Verify",
                    "GetSim",
                    "GetImsi",
                    "Radio",
                    "Preferred",
                    "Allow",
                    "Status",
                    "Uicc",
                    "Card",
                    "Slot",
                    "ATR",
                    "Channel",
                    "Facility",
                    "Auto",
                    "Pin",
                    "Request",
                    "Send",
                    "IoChannel",
                )
            )
            if interesting:
                print(" ", n)

# Empty / nearly-empty GETs from sit-stream that are SIM-adjacent
print("\n=== Empty-ish SIM/NET GETs (possible missing live) ===")
cands = [
    "BuildSim3GPbCapa",
    "BuildSimGetPbStorageInfo",
    "BuildGetVoiceOperation",
    "BuildGetPreferredCallCapability",
    "BuildGetPsService",
    "BuildGetDataCallList",
    "BuildGetPhoneCapability",
    "BuildSimGetSlotStatus",
    "BuildSimGetATR",
    "BuildGetRadioState",
    "BuildQueryNetworkSelectionMode",
    "BuildOperator",
    "BuildRadioConfigReset",
    "BuildSetVoiceOperation",
    "BuildSetMobileDataState",
    "BuildSetPreferredDataModem",
    "BuildSimReadPbEntry",
]
for needle in cands:
    hits = [(k, v) for k, v in ssyms.items() if needle in k]
    for k, v in hits:
        oid, olen = decode_id_len(stream, v)
        print(f"  {hex(v)} id={oid and hex(oid)} len={olen}  {k[:100]}")

# DoGetImsi: does it call BuildSimIO?
print("\n=== DoGetImsi BL → ===")
for k, v in rsyms.items():
    if "DoGetImsi" in k:
        for t in bl_targets(ril, v, 0x300):
            nk = ril_by_va.get(t)
            if nk:
                print(" ", nk[:120])

# What does OnVerifyPinDone call?
print("\n=== OnVerifyPin / OnVerifyPinDone ===")
for k, v in sorted(rsyms.items(), key=lambda x: x[1]):
    if "VerifyPin" in k and ("On" in k or "Do" in k):
        print(f"SYM {hex(v)} {k[:120]}")
        for t in bl_targets(ril, v, 0x280):
            nk = ril_by_va.get(t)
            if nk and any(
                x in nk
                for x in (
                    "GetSim",
                    "GetImsi",
                    "Build",
                    "Status",
                    "Request",
                    "Send",
                    "Radio",
                    "Allow",
                    "Preferred",
                )
            ):
                print("  BL", nk[:120])
