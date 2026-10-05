#!/usr/bin/env python3
"""RE post-VerifyPin OpenChannel / TransmitApdu / SIM_IO SELECT path (no secrets)."""
from __future__ import annotations

import struct
from pathlib import Path

import sys

CANDIDATES = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"),
    Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics"),
]
DIAG = next((p for p in CANDIDATES if (p / "libsitril.so").is_file()), None)
if DIAG is None:
    print("libsitril.so not found", file=sys.stderr)
    sys.exit(2)
ril = (DIAG / "libsitril.so").read_bytes()
stream = (DIAG / "sit-stream.so").read_bytes()


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def u64(b, o):
    return struct.unpack_from("<Q", b, o)[0]


def cstr(b, o, n=240):
    s = bytearray()
    for i in range(o, min(len(b), o + n)):
        if b[i] == 0:
            break
        if 32 <= b[i] < 127:
            s.append(b[i])
        else:
            break
    return s.decode() if s else ""


def loads_of(blob):
    e_phoff = u64(blob, 0x20)
    e_phentsize = u16(blob, 0x36)
    e_phnum = u16(blob, 0x38)
    out = []
    for i in range(e_phnum):
        o = e_phoff + i * e_phentsize
        if u32(blob, o) != 1:
            continue
        out.append((u64(blob, o + 8), u64(blob, o + 16), u64(blob, o + 32)))
    return out


def va_to_file(loads, va):
    for po, pv, psz in loads:
        if pv <= va < pv + psz:
            return po + (va - pv)
    return None


def dynsyms(blob):
    e_shoff = u64(blob, 0x28)
    e_shentsize = u16(blob, 0x3A)
    e_shnum = u16(blob, 0x3C)
    e_shstrndx = u16(blob, 0x3E)
    shstr = u64(blob, e_shoff + e_shstrndx * e_shentsize + 0x18)
    dynsym = dynstr = None
    for i in range(e_shnum):
        off = e_shoff + i * e_shentsize
        name = cstr(blob, shstr + u32(blob, off))
        if name == ".dynsym":
            dynsym = (u64(blob, off + 0x18), u64(blob, off + 0x20), u64(blob, off + 0x38) or 24)
        if name == ".dynstr":
            dynstr = (u64(blob, off + 0x18), u64(blob, off + 0x20))
    if not dynsym or not dynstr:
        return {}
    loads = loads_of(blob)
    out = {}
    ent = dynsym[2]
    for i in range(0, dynsym[1], ent):
        o = dynsym[0] + i
        name = cstr(blob, dynstr[0] + u32(blob, o), 320)
        st_value = u64(blob, o + 8)
        st_size = u64(blob, o + 16)
        if not name or not st_value:
            continue
        fo = va_to_file(loads, st_value)
        out[name] = (st_value, st_size, fo)
    return out


def bl_targets(blob, loads, func_va, size=0x500):
    off = va_to_file(loads, func_va)
    if off is None:
        return []
    chunk = blob[off : off + size]
    out = []
    for i in range(0, len(chunk) - 3, 4):
        insn = u32(chunk, i)
        if (insn & 0xFC000000) != 0x94000000:
            continue
        imm26 = insn & 0x03FFFFFF
        if imm26 & 0x02000000:
            imm26 -= 0x04000000
        target = func_va + i + (imm26 << 2)
        out.append((func_va + i, target))
    return out


def movz_ids(blob, loads, func_va, size=0x400):
    off = va_to_file(loads, func_va)
    if off is None:
        return []
    chunk = blob[off : off + size]
    ids = []
    for i in range(0, len(chunk) - 3, 4):
        insn = u32(chunk, i)
        if (insn & 0xFF800000) != 0x52800000:
            continue
        imm16 = (insn >> 5) & 0xFFFF
        hw = (insn >> 21) & 3
        val = imm16 << (hw * 16)
        if 0x200 <= val <= 0x2FF or 0x600 <= val <= 0x9FF:
            ids.append((func_va + i, val))
    return ids


def decode_layout(blob, fo, nins=80):
    """Compact decode for builder layouts."""
    lines = []
    for i in range(fo, min(len(blob) - 3, fo + nins * 4), 4):
        w = u32(blob, i)
        if w == 0xD65F03C0:
            lines.append((i, "RET"))
            break
        if (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            lines.append((i, f"MOVZ w{rd},#{hex(imm16 << (hw * 16))}"))
            continue
        if (w & 0xFFC00000) == 0x39000000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = (w >> 10) & 0xFFF
            lines.append((i, f"STRB w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFFC00000) == 0xB9000000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = ((w >> 10) & 0xFFF) * 4
            lines.append((i, f"STR w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFC000000) == 0x94000000:
            imm26 = w & 0x3FFFFFF
            if imm26 & (1 << 25):
                imm26 -= 1 << 26
            lines.append((i, f"BL {hex(i + imm26 * 4)}"))
            continue
    return lines


rsym = dynsyms(ril)
ssym = dynsyms(stream)
rloads = loads_of(ril)
sloads = loads_of(stream)
ril_by_va = {v[0]: k for k, v in rsym.items()}

print("=== OnVerifyPinDone / related BL graph ===")
needles = [
    "OnVerifyPinDone",
    "DoVerifyPin",
    "OpenSimChannel",
    "DoOpenChannel",
    "TransmitApdu",
    "DoSimIo",
    "DoTransmit",
    "OnOpenChannel",
    "SimIoService",
]
for name, (va, sz, fo) in sorted(rsym.items(), key=lambda x: x[1][0]):
    if not any(n in name for n in needles):
        continue
    if "VerifyPin2" in name and "OnVerifyPinDone" not in name and "DoVerifyPin" not in name:
        continue
    short = name if len(name) < 120 else name[:117] + "..."
    print(f"\n# {short}")
    print(f"  va={hex(va)} size={sz} fo={fo and hex(fo)}")
    for src, tgt in bl_targets(ril, rloads, va, max(sz or 0x300, 0x300)):
        nk = ril_by_va.get(tgt)
        if not nk:
            continue
        if any(
            x in nk
            for x in (
                "Open",
                "Channel",
                "Transmit",
                "SimIo",
                "SIM_IO",
                "GetSim",
                "GetImsi",
                "Build",
                "Request",
                "Send",
                "Status",
                "Select",
                "Aid",
                "AID",
                "Card",
                "Uicc",
                "Facility",
                "ATR",
                "Verify",
                "Radio",
                "Allow",
                "Preferred",
            )
        ):
            print(f"  BL {hex(src)} -> {nk[:140]}")
    ids = movz_ids(ril, rloads, va, max(sz or 0x200, 0x200))
    if ids:
        print("  MOVZ ids:", ", ".join(f"{hex(a)}={hex(v)}" for a, v in ids[:12]))

print("\n=== sit-stream builder layouts (OpenChannelWithP2 / TransmitChannel / SimIO head) ===")
for key in (
    "BuildSimOpenChannelWithP2",
    "BuildSimOpenChannel",
    "BuildSimTransmitApduChannel",
    "BuildSimTransmitApduBasic",
    "BuildSimIO",
    "BuildSimCloseChannel",
):
    hits = [(n, v) for n, v in ssym.items() if key in n]
    for n, (va, sz, fo) in hits:
        print(f"\n## {n}")
        print(f"   va={hex(va)} fo={fo and hex(fo)} size={sz}")
        if fo is None:
            continue
        for i, s in decode_layout(stream, fo, min(90, (sz or 200) // 4 + 10)):
            if any(x in s for x in ("MOVZ", "STR", "BL ", "RET")):
                print(f"   {hex(i)} {s}")

print("\n=== libsitril OpenSimChannelHandler / Oem path strings ===")
for nb in [
    b"OpenSimChannel",
    b"OpenChannelWithP2",
    b"TransmitApduChannel",
    b"SIM_IO",
    b"SelectAid",
    b"SELECT",
    b"OnVerifyPinDone",
    b"GetSimStatus",
]:
    pos = 0
    c = 0
    while c < 6:
        i = ril.find(nb, pos)
        if i < 0:
            break
        print(f"  @{hex(i)} {cstr(ril, i, 160)}")
        pos = i + 1
        c += 1

print("\nDONE")
