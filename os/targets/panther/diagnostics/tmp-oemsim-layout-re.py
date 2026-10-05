#!/usr/bin/env python3
"""RE BuildOemSimRequest switch + named layouts for 0x208/0x20c/0x20f/0x247."""
from __future__ import annotations

import struct
from pathlib import Path

MR = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
SS = (MR / "sit-stream.so").read_bytes()
LS = (MR / "libsitril.so").read_bytes()


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def u64(b, o):
    return struct.unpack_from("<Q", b, o)[0]


def cstr(b, o, n=200):
    s = bytearray()
    for i in range(o, min(len(b), o + n)):
        if b[i] == 0:
            break
        if b[i] < 32 or b[i] > 126:
            break
        s.append(b[i])
    return s.decode() if s else ""


def expand(blob, h, maxlen=180):
    a = h
    while a > 0 and 32 <= blob[a - 1] < 127:
        a -= 1
    b = h
    while b < len(blob) and blob[b] and b - a < maxlen:
        b += 1
    return blob[a:b].decode("ascii", "ignore")


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


def file_to_va(loads, fo):
    for po, pv, psz in loads:
        if po <= fo < po + psz:
            return pv + (fo - po)
    return None


def dynsyms(blob, pred):
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
        return []
    loads = loads_of(blob)
    out = []
    ent = dynsym[2]
    for i in range(0, dynsym[1], ent):
        o = dynsym[0] + i
        name = cstr(blob, dynstr[0] + u32(blob, o), 300)
        st_value = u64(blob, o + 8)
        st_size = u64(blob, o + 16)
        if not name or not pred(name):
            continue
        fo = va_to_file(loads, st_value)
        out.append((fo, st_size, name, st_value))
    return out


def decode(blob, start, nins=120):
    out = []
    for i in range(start, min(len(blob) - 3, start + nins * 4), 4):
        w = u32(blob, i)
        if w == 0xD65F03C0:
            out.append((i, "RET"))
            break
        if (w & 0x7F800000) == 0x71000000:
            rn = (w >> 5) & 0x1F
            imm = (w >> 10) & 0xFFF
            sf = (w >> 31) & 1
            out.append((i, f"CMP {'x' if sf else 'w'}{rn},#{imm}"))
            continue
        if (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            out.append((i, f"MOVZ w{rd},#{hex(imm16 << (hw * 16))}"))
            continue
        if (w & 0xFF800000) == 0x72800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            out.append((i, f"MOVK w{rd},#{hex(imm16)},LSL#{hw * 16}"))
            continue
        if (w & 0xFF000000) == 0x54000000:
            imm19 = (w >> 5) & 0x7FFFF
            if imm19 & (1 << 18):
                imm19 -= 1 << 19
            conds = [
                "EQ",
                "NE",
                "CS",
                "CC",
                "MI",
                "PL",
                "VS",
                "VC",
                "HI",
                "LS",
                "GE",
                "LT",
                "GT",
                "LE",
                "AL",
                "NV",
            ]
            out.append((i, f"B.{conds[w & 0xF]} {hex(i + imm19 * 4)}"))
            continue
        if (w & 0xFC000000) == 0x14000000:
            imm26 = w & 0x3FFFFFF
            if imm26 & (1 << 25):
                imm26 -= 1 << 26
            out.append((i, f"B {hex(i + imm26 * 4)}"))
            continue
        if (w & 0xFC000000) == 0x94000000:
            imm26 = w & 0x3FFFFFF
            if imm26 & (1 << 25):
                imm26 -= 1 << 26
            out.append((i, f"BL {hex(i + imm26 * 4)}"))
            continue
        if (w & 0x7FE0FFE0) == 0x2A0003E0:
            rd = w & 0x1F
            rm = (w >> 16) & 0x1F
            sf = (w >> 31) & 1
            r = "x" if sf else "w"
            out.append((i, f"MOV {r}{rd},{r}{rm}"))
            continue
        if (w & 0x7F000000) == 0x11000000:
            rd = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = (w >> 10) & 0xFFF
            sh = (w >> 22) & 1
            sf = (w >> 31) & 1
            op = "ADD" if ((w >> 30) & 1) == 0 else "SUB"
            r = "x" if sf else "w"
            out.append((i, f"{op} {r}{rd},{r}{rn},#{imm12 << (12 * sh)}"))
            continue
        if (w & 0xFFC00000) == 0xB9400000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = ((w >> 10) & 0xFFF) * 4
            out.append((i, f"LDR w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFFC00000) == 0xF9400000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = ((w >> 10) & 0xFFF) * 8
            out.append((i, f"LDR x{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFFC00000) == 0xB9000000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = ((w >> 10) & 0xFFF) * 4
            out.append((i, f"STR w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFFC00000) == 0x39400000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = (w >> 10) & 0xFFF
            out.append((i, f"LDRB w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0xFFC00000) == 0x39000000:
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm = (w >> 10) & 0xFFF
            out.append((i, f"STRB w{rt},[x{rn},#{imm}]"))
            continue
        if (w & 0x7E000000) == 0x34000000:
            rt = w & 0x1F
            imm19 = (w >> 5) & 0x7FFFF
            is64 = (w >> 31) & 1
            op = (w >> 24) & 1
            if imm19 & (1 << 18):
                imm19 -= 1 << 19
            r = "x" if is64 else "w"
            out.append((i, f'{"CBNZ" if op else "CBZ"} {r}{rt},{hex(i + imm19 * 4)}'))
            continue
        # SUBS (for CMP alias already covered); ORR imm
        out.append((i, f"?{hex(w)}"))
    return out


def interesting(s):
    keys = (
        "MOVZ",
        "MOVK",
        "CMP",
        "B.",
        "B ",
        "BL ",
        "STR",
        "LDR",
        "RET",
        "MOV ",
        "ADD ",
        "SUB ",
        "CBZ",
        "CBNZ",
    )
    return any(s.startswith(k) or k in s for k in keys)


def main():
    print("=== BuildOemSimRequest full @0x806d0 ===")
    for i, s in decode(SS, 0x806D0, 100):
        print(f"  {hex(i)} {s}")

    print("\n=== MOVZ #0x208 / #0x20c / #0x20f / #0x247 ownership ===")
    loads = loads_of(SS)
    syms = dynsyms(SS, lambda n: "ProtocolSimBuilder" in n and n.startswith("_ZN"))
    # also fix fo
    fixed = []
    for fo, sz, name, va in syms:
        if fo is None:
            fo = va_to_file(loads, va)
        fixed.append((fo, sz, name, va))
    fixed = [x for x in fixed if x[0] is not None]
    fixed.sort()

    for imm in (0x208, 0x20C, 0x20F, 0x247, 0x20D, 0x200, 0x201):
        hits = []
        for i in range(0, len(SS) - 3, 4):
            w = u32(SS, i)
            if (w & 0xFF800000) != 0x52800000:
                continue
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            if imm16 << (hw * 16) == imm:
                hits.append(i)
        print(f"\nMOVZ #{hex(imm)} x{len(hits)}: {[hex(h) for h in hits[:12]]}")
        for h in hits:
            owner = None
            for fo, sz, name, va in fixed:
                if fo <= h < fo + max(sz, 4):
                    owner = (fo, sz, name)
                    break
            if owner:
                print(f"  {hex(h)} -> {owner[2]} @{hex(owner[0])} size={owner[1]}")
            else:
                print(f"  {hex(h)} -> NO_DYNSYM_OWNER")

    want = [
        "BuildOemSimRequest",
        "BuildSimTransmitApduBasic",
        "BuildSimTransmitApduChannel",
        "BuildSimOpenChannelWithP2",
        "BuildSimOpenChannel",
        "BuildGetImsi",
        "BuildSimGetGbaAuth",
        "BuildSimCloseChannel",
        "BuildSimGetSimAuth",
        "BuildSetSimCardPower",
    ]
    for fo, sz, name, va in fixed:
        if not any(w in name for w in want):
            continue
        print(f"\n=== {name} fo={hex(fo)} size={sz} ===")
        for i, s in decode(SS, fo, min(100, sz // 4 + 8)):
            if interesting(s):
                print(f"  {hex(i)} {s}")

    print("\n=== libsitril OemSimAuth / CreateOemSim strings ===")
    for nb in [
        b"CreateOemSimAuthRequest",
        b"OemSimAuthRequest",
        b"BuildOemSimRequest",
        b"OemSim",
        b"SIM_IO",
        b"DoOemSim",
        b"OnOemSim",
        b"SIT_SIM_",
    ]:
        pos = 0
        c = 0
        while c < 8:
            i = LS.find(nb, pos)
            if i < 0:
                break
            print(f"  @{hex(i)} {expand(LS, i)[:160]}")
            pos = i + 1
            c += 1

    # Hunt CreateOemSimAuthRequest body in libsitril via mangled
    mang = b"_ZN17RilRequestCreator23CreateOemSimAuthRequestEiPvPcj"
    i = LS.find(mang)
    print(f"\nCreateOemSimAuthRequest mangled @{hex(i) if i>=0 else -1}")
    # Find BL to BuildOemSimRequest in sit-stream by scanning for BL to 0x806d0
    print("\n=== sit-stream BL targets to BuildOemSimRequest (0x806d0) ===")
    target = 0x806D0
    bls = []
    for i in range(0, len(SS) - 3, 4):
        w = u32(SS, i)
        if (w & 0xFC000000) != 0x94000000:
            continue
        imm26 = w & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        if i + imm26 * 4 == target:
            bls.append(i)
    print([hex(h) for h in bls])
    for h in bls:
        # find owning function
        owner = None
        for fo, sz, name, va in fixed:
            if fo <= h < fo + max(sz, 4):
                owner = name
                break
        print(f"  BL @{hex(h)} owner={owner}")
        # decode 30 ins before
        start = max(0, h - 80)
        for i, s in decode(SS, start, 30):
            if interesting(s):
                print(f"    {hex(i)} {s}")

    # Also search string table near OemSim for RIL request ids
    print("\n=== sit-stream strings near OemSim / SIM_IO / Transmit ===")
    for nb in [b"OemSim", b"SIM_IO", b"TransmitApdu", b"OpenChannel", b"GetImsi", b"SIM_AUTH"]:
        pos = 0
        c = 0
        while c < 6:
            i = SS.find(nb, pos)
            if i < 0:
                break
            print(f"  @{hex(i)} {expand(SS, i)[:140]}")
            pos = i + 1
            c += 1

    print("DONE")


if __name__ == "__main__":
    main()
