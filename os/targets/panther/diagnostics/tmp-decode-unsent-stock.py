#!/usr/bin/env python3
"""Decode unsent stock builders + OnSimHotSwap / DualNetwork call chains."""
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

    names = data[sh(e_shstrndx)["off"] : sh(e_shstrndx)["off"] + sh(e_shstrndx)["size"]]
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


def decode(data: bytes, addr: int, window: int = 0x140):
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
            if 0x0200 <= val <= 0x02FF or 0x0400 <= val <= 0x09FF or 0x4600 <= val <= 0x46FF:
                ids.append(val)
            if 8 <= val <= 300:
                lens.append(val)
    return (
        ids[0] if ids else None,
        next((L for L in lens if L in (12, 13, 14, 15, 16, 17, 18, 21, 24)), lens[0] if lens else None),
        movs[:20],
    )


def dump_builder(data: bytes, addr: int, n: int = 0xA0):
    off = va_to_off(data, addr)
    if off is None:
        return
    chunk = data[off : off + n]
    for i in range(0, len(chunk) - 3, 4):
        insn = struct.unpack_from("<I", chunk, i)[0]
        # MOVZ W
        if (insn & 0x7F800000) == 0x52800000:
            imm16 = (insn >> 5) & 0xFFFF
            rd = insn & 0x1F
            print(f"  +{i:03x}: MOVZ W{rd},#{hex(imm16)}")
        # STRB Wt,[Xn,#imm]
        elif (insn & 0xFFC00000) == 0x39000000:
            imm = (insn >> 10) & 0xFFF
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STRB W{rt},[X{rn},#{imm}]")
        # STRH
        elif (insn & 0xFFC00000) == 0x79000000:
            imm = ((insn >> 10) & 0xFFF) << 1
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STRH W{rt},[X{rn},#{imm}]")
        # STR W
        elif (insn & 0xFFC00000) == 0xB9000000:
            imm = ((insn >> 10) & 0xFFF) << 2
            rn = (insn >> 5) & 0x1F
            rt = insn & 0x1F
            print(f"  +{i:03x}: STR W{rt},[X{rn},#{imm}]")


ss = parse_dynsym(stream)
rs = parse_dynsym(ril)

print("=== candidate builders ===")
needles = (
    "MobileData",
    "DualNetwork",
    "VoiceOperation",
    "PreferredCall",
    "PreferredDataModem",
    "GetIMEI",
    "GetDevID",
    "GetNitz",
    "GetCellInfo",
    "GetEndc",
    "GetNrMode",
    "AllowedNetwork",
    "LtePreferred",
    "GetPhoneCapability",
    "RadioConfigReset",
    "GetFrequency",
    "GetDuplex",
    "GetManualRat",
    "GetRCNetwork",
    "GetBarring",
    "GetVonr",
    "GetDeviceService",
)
for needle in needles:
    for k, v in sorted(ss.items(), key=lambda x: x[1]):
        if needle.lower() in k.lower():
            oid, olen, movs = decode(stream, v)
            print(
                f"  {hex(v)} id={oid and hex(oid)} len={olen} "
                f"movs={[hex(x) for x in movs[:8]]} {k[:95]}"
            )

print("\n=== BuildSetMobileDataState dump ===")
for k, v in ss.items():
    if "BuildSetMobileDataState" in k:
        print(k, hex(v))
        dump_builder(stream, v, 0xC0)

print("\n=== BuildGetPreferredCallCapability dump ===")
for k, v in ss.items():
    if "BuildGetPreferredCallCapability" in k:
        print(k, hex(v))
        dump_builder(stream, v, 0x80)

print("\n=== BuildSetVoiceOperation dump ===")
for k, v in ss.items():
    if "BuildSetVoiceOperation" in k:
        print(k, hex(v))
        dump_builder(stream, v, 0xA0)

print("\n=== DualNetwork symbols in libsitril ===")
for k, v in sorted(rs.items(), key=lambda x: x[1]):
    if "DualNetwork" in k or "MobileDataState" in k or "LtePreferred" in k:
        print(hex(v), k[:130])

# strings near DualNetwork for opcode hints
print("\n=== DualNetwork / MobileData strings ===")
for needle in (
    b"DualNetwork",
    b"MobileDataState",
    b"SetDualNetwork",
    b"preferred call",
    b"PreferredCall",
    b"0x0625",
    b"0x091a",
    b"0x0930",
):
    i = 0
    c = 0
    while c < 5:
        j = ril.find(needle, i)
        if j < 0:
            break
        end = ril.find(b"\0", j)
        print(hex(j), ril[j:end][:120])
        i = j + 1
        c += 1

print("\n=== empty GETs never clearly live-tried (from inventory) ===")
for name, expect_id in (
    ("BuildGetPreferredCallCapability", 0x930),
    ("BuildGetPhoneCapability", 0x615),
    ("BuildGetDataCallList", 0x602),
    ("BuildSim3GPbCapa", 0x245),
    ("BuildGetDuplexMode", None),
    ("BuildGetManualRatMode", None),
    ("BuildGetNrMode", None),
    ("BuildGetEndcMode", None),
    ("BuildGetVonrCapa", None),
    ("BuildGetNitzTime", None),
    ("BuildGetDeviceService", None),
    ("BuildGetAllowedNetworkTypeBitmap", None),
    ("BuildQueryAvailableBandMode", None),
    ("BuildGetRCNetworkType", None),
    ("BuildGetCellInfoList", None),
    ("BuildGetBarringInfo", None),
    ("BuildGetFrequencyInfo", None),
):
    hits = [(k, v) for k, v in ss.items() if name in k]
    for k, v in hits:
        oid, olen, movs = decode(stream, v)
        print(f"  {name}: id={oid and hex(oid)} len={olen} expect={expect_id and hex(expect_id)}")
