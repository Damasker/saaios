#!/usr/bin/env python3
"""Disasm ProtocolSimBuilder::BuildOemSimRequest + libsitril 0x2f50 registry use."""
from __future__ import annotations

import struct
from pathlib import Path

MR = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
SS = (MR / "sit-stream.so").read_bytes()
LS = (MR / "libsitril.so").read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def u64(b, o):
    return struct.unpack_from("<Q", b, o)[0]


def find_all(hay, needle, limit=40):
    out, pos = [], 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def expand(blob, h, maxlen=180):
    a = h
    while a > 0 and 32 <= blob[a - 1] < 127:
        a -= 1
    b = h
    while b < len(blob) and blob[b] and b - a < maxlen:
        b += 1
    return blob[a:b].decode("ascii", "ignore")


def decode_a64(blob, start, nins=80):
    """Minimal AArch64 decode for immediates of interest."""
    out = []
    for i in range(start, min(len(blob) - 3, start + nins * 4), 4):
        w = u32(blob, i)
        # RET
        if w == 0xD65F03C0:
            out.append((i, "RET"))
            break
        # MOVZ Wd,#imm16
        if (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            out.append((i, f"MOVZ w{rd},#{hex(imm16 << (hw * 16))}"))
            continue
        # MOVK
        if (w & 0xFF800000) == 0x72800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            out.append((i, f"MOVK w{rd},#{hex(imm16)},LSL#{hw * 16}"))
            continue
        # MOVN
        if (w & 0xFF800000) == 0x12800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            out.append((i, f"MOVN w{rd},#{hex(imm16 << (hw * 16))}"))
            continue
        # ADD/SUB imm
        if (w & 0x7F000000) == 0x11000000:
            rd = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = (w >> 10) & 0xFFF
            sh = (w >> 22) & 1
            sf = (w >> 31) & 1
            op = "ADD" if ((w >> 30) & 1) == 0 else "SUB"
            reg = "x" if sf else "w"
            out.append((i, f"{op} {reg}{rd},{reg}{rn},#{imm12 << (12 * sh)}"))
            continue
        # LDRB/LDRH/STRB unsigned imm
        if (w & 0xFFC00000) == 0x39400000:  # LDRB
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = (w >> 10) & 0xFFF
            out.append((i, f"LDRB w{rt},[x{rn},#{imm12}]"))
            continue
        if (w & 0xFFC00000) == 0x39000000:  # STRB
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = (w >> 10) & 0xFFF
            out.append((i, f"STRB w{rt},[x{rn},#{imm12}]"))
            continue
        if (w & 0xFFC00000) == 0x79400000:  # LDRH
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = ((w >> 10) & 0xFFF) * 2
            out.append((i, f"LDRH w{rt},[x{rn},#{imm12}]"))
            continue
        if (w & 0xFFC00000) == 0x79000000:  # STRH
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = ((w >> 10) & 0xFFF) * 2
            out.append((i, f"STRH w{rt},[x{rn},#{imm12}]"))
            continue
        if (w & 0xFFC00000) == 0xB9000000:  # STR W
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = ((w >> 10) & 0xFFF) * 4
            out.append((i, f"STR w{rt},[x{rn},#{imm12}]"))
            continue
        if (w & 0xFFC00000) == 0xB9400000:  # LDR W
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            imm12 = ((w >> 10) & 0xFFF) * 4
            out.append((i, f"LDR w{rt},[x{rn},#{imm12}]"))
            continue
        # BL
        if (w & 0xFC000000) == 0x94000000:
            imm26 = w & 0x3FFFFFF
            if imm26 & (1 << 25):
                imm26 -= 1 << 26
            target = i + imm26 * 4
            out.append((i, f"BL {hex(target)}"))
            continue
        # ORR immediate / MOV alias Wd,Wm: 2A0003E0 pattern
        if (w & 0x7FE0FFE0) == 0x2A0003E0:
            rd = w & 0x1F
            rm = (w >> 16) & 0x1F
            sf = (w >> 31) & 1
            reg = "x" if sf else "w"
            out.append((i, f"MOV {reg}{rd},{reg}{rm}"))
            continue
        out.append((i, f"? {hex(w)}"))
    return out


def symbol_file_off(blob, mangled: bytes):
    """Find GOT/PLT-ish: locate mangled string then look for code xref via ADRP patterns — fallback: find function start via nearby code after .dynsym.

    For stripped .so we use: find string in dynstr, then search for pointer to it? Simpler: use known prior offset from strings listing if present in .text via scanning for unique MOVZ sequences after locating via Capstone-less heuristic:

    Actually sit-stream exports? Try dynsym.
    """
    # ELF dynsym scan
    if blob[:4] != b"\x7fELF":
        return None
    e_phoff = u64(blob, 0x20) if blob[4] == 2 else u32(blob, 0x1C)
    e_phentsize = u16(blob, 0x36) if blob[4] == 2 else u16(blob, 0x2A)
    e_phnum = u16(blob, 0x38) if blob[4] == 2 else u16(blob, 0x2C)
    # Use section headers
    e_shoff = u64(blob, 0x28)
    e_shentsize = u16(blob, 0x3A)
    e_shnum = u16(blob, 0x3C)
    e_shstrndx = u16(blob, 0x3E)
    shstr = u64(blob, e_shoff + e_shstrndx * e_shentsize + 0x18)
    dynsym = dynstr = None
    for i in range(e_shnum):
        off = e_shoff + i * e_shentsize
        name_off = u32(blob, off)
        name = cstr_raw(blob, shstr + name_off)
        if name == ".dynsym":
            dynsym = (u64(blob, off + 0x18), u64(blob, off + 0x20), u64(blob, off + 0x38) or 24)
        if name == ".dynstr":
            dynstr = (u64(blob, off + 0x18), u64(blob, off + 0x20))
        if name == ".symtab":
            pass
    if not dynsym or not dynstr:
        return None
    ent_sz = dynsym[2]
    for i in range(0, dynsym[1], ent_sz):
        o = dynsym[0] + i
        st_name = u32(blob, o)
        st_info = blob[o + 4]
        st_value = u64(blob, o + 8)
        st_size = u64(blob, o + 16)
        name = cstr_raw(blob, dynstr[0] + st_name)
        if name and mangled.decode() in name:
            return st_value, st_size, name
    return None


def cstr_raw(b, o, n=200):
    s = bytearray()
    for i in range(o, min(len(b), o + n)):
        if b[i] == 0:
            break
        if b[i] < 32 or b[i] > 126:
            break
        s.append(b[i])
    return s.decode() if s else ""


def main():
    # Locate BuildOemSimRequest string and function
    targets = [
        b"_ZN18ProtocolSimBuilder18BuildOemSimRequestEiPhi",
        b"BuildOemSimRequest",
        b"_ZN18ProtocolSimBuilder19BuildSimVerifyPin",
        b"BuildSimVerifyPin",
        b"BuildSimGetStatus",
        b"BuildSimInit",
    ]
    log("=== sit-stream symbol / string ===")
    for t in targets:
        hits = find_all(SS, t, 5)
        log(f"  {t!r} {[hex(h) for h in hits]}")
        for h in hits[:2]:
            log(f"    {expand(SS, h)}")

    # dynsym lookup
    for mang in [
        b"_ZN18ProtocolSimBuilder18BuildOemSimRequestEiPhi",
        b"_ZN18ProtocolSimBuilder18BuildSimVerifyPinEPhi",
        b"_ZN18ProtocolSimBuilder19BuildSimVerifyPin2EPhi",
        b"_ZN18ProtocolSimBuilder15BuildSimGetImsiEv",
        b"_ZN18ProtocolSimBuilder17BuildSimGetStatusEv",
    ]:
        r = symbol_file_off(SS, mang)
        log(f"dynsym {mang.decode()}: {r}")
        if r:
            val, size, name = r
            # For ET_DYN, st_value is often virt addr; file offset ~= value for Android .so with vaddr=0 load
            # Find PT_LOAD
            e_phoff = u64(SS, 0x20)
            e_phentsize = u16(SS, 0x36)
            e_phnum = u16(SS, 0x38)
            file_off = None
            for i in range(e_phnum):
                o = e_phoff + i * e_phentsize
                p_type = u32(SS, o)
                if p_type != 1:
                    continue
                p_offset = u64(SS, o + 8)
                p_vaddr = u64(SS, o + 16)
                p_filesz = u64(SS, o + 32)
                if p_vaddr <= val < p_vaddr + p_filesz:
                    file_off = p_offset + (val - p_vaddr)
                    break
            log(f"  file_off={hex(file_off) if file_off else None} size={size}")
            if file_off is not None:
                log(f"  bytes: {SS[file_off:file_off+32].hex()}")
                for i, s in decode_a64(SS, file_off, 100):
                    if s.startswith("?") and "MOVZ" not in s and "STR" not in s and "LDR" not in s and "BL" not in s and "ADD" not in s and "SUB" not in s and "RET" not in s and "MOV " not in s:
                        continue
                    log(f"    {hex(i)} {s}")

    # Same for libsitril (often duplicates)
    log("\n=== libsitril BuildOemSimRequest ===")
    mang = b"_ZN18ProtocolSimBuilder18BuildOemSimRequestEiPhi"
    hits = find_all(LS, mang, 3)
    log(f"str hits {[hex(h) for h in hits]}")
    r = symbol_file_off(LS, mang)
    log(f"dynsym: {r}")
    if r:
        val, size, name = r
        e_phoff = u64(LS, 0x20)
        e_phentsize = u16(LS, 0x36)
        e_phnum = u16(LS, 0x38)
        file_off = None
        for i in range(e_phnum):
            o = e_phoff + i * e_phentsize
            if u32(LS, o) != 1:
                continue
            p_offset = u64(LS, o + 8)
            p_vaddr = u64(LS, o + 16)
            p_filesz = u64(LS, o + 32)
            if p_vaddr <= val < p_vaddr + p_filesz:
                file_off = p_offset + (val - p_vaddr)
                break
        log(f"  file_off={hex(file_off) if file_off else None}")
        if file_off is not None:
            for i, s in decode_a64(LS, file_off, 120):
                interesting = any(
                    k in s
                    for k in (
                        "MOVZ",
                        "MOVK",
                        "STR",
                        "LDR",
                        "BL",
                        "ADD",
                        "SUB",
                        "RET",
                        "MOV ",
                    )
                )
                if interesting:
                    log(f"    {hex(i)} {s}")

    # Who calls BuildOemSimRequest? search BL targets later via string xrefs in RIL
    log("\n=== callers / related strings ===")
    for blob, label in ((SS, "sit-stream"), (LS, "libsitril")):
        for nb in [
            b"BuildOemSimRequest",
            b"OEM_SIM",
            b"SIT_OEM",
            b"SIM_INIT",
            b"0x2f50",
            b"OemSim",
            b"DoOemSim",
            b"OnOemSim",
        ]:
            hs = find_all(blob, nb, 20)
            if hs:
                log(f"{label} {nb!r} x{len(hs)}")
                for h in hs[:6]:
                    log(f"  @{hex(h)} {expand(blob, h)[:140]}")

    # Registry neighborhood decode around 0x95660
    log("\n=== libsitril msgid table neighborhood ===")
    base = 0x95660
    for i in range(-5, 12):
        o = base + i * 24
        mid = u16(LS, o)
        typ = u16(LS, o + 2)
        a = u32(LS, o + 4)
        b = u32(LS, o + 8)
        c = u32(LS, o + 12)
        log(f"  @{hex(o)} id={hex(mid)} typ={hex(typ)} a={hex(a)} b={hex(b)} c={hex(c)} raw={LS[o:o+24].hex()}")

    # Search SIT header builder: look for stores of type/pad/id/len/token pattern
    # soft VerifyPin uses id 0x0201 — find Build that MOVZ 0x201
    log("\n=== libsitril MOVZ #0x201 / #0x200 (SIT SIM) near builders ===")
    for imm, name in [(0x201, "VerifyPin?"), (0x200, "GetStatus?"), (0x2F50, "OEM INIT"), (0x2F52, "OEM VP")]:
        hits = []
        for i in range(0, len(LS) - 3, 4):
            w = u32(LS, i)
            if (w & 0xFF800000) != 0x52800000:
                continue
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            if imm16 << (hw * 16) == imm:
                hits.append(i)
                if len(hits) >= 8:
                    break
        log(f"  MOVZ #{hex(imm)} ({name}): {[hex(h) for h in hits]}")

    log("DONE")


if __name__ == "__main__":
    main()
