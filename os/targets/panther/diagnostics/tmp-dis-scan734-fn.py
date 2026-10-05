#!/usr/bin/env python3
"""Linear AArch64 dump of TD1A BuildStartNetworkScan (va 0x2376a0, size 0x62c)."""
import struct
from pathlib import Path

SO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so")
VA = 0x2376A0
SIZE = 0x62C


def parse_sections(data):
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]
    sections = []
    for i in range(e_shnum):
        o = e_shoff + i * e_shentsize
        sections.append({
            "name": struct.unpack_from("<I", data, o)[0],
            "flags": struct.unpack_from("<Q", data, o + 8)[0],
            "addr": struct.unpack_from("<Q", data, o + 16)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
        })
    shstr = sections[e_shstrndx]
    for s in sections:
        o = shstr["off"] + s["name"]
        s["sname"] = data[o:data.find(b"\x00", o)].decode()
    return sections


def va_off(sections, va):
    for s in sections:
        if s["addr"] <= va < s["addr"] + max(s["size"], 1):
            return s["off"] + (va - s["addr"])
    raise SystemExit(f"va 0x{va:x} not in a section")


def reg(n, sf):
    if n == 31:
        return "sp" if False else ("xzr" if sf else "wzr")
    return f"{'x' if sf else 'w'}{n}"


def dis(w, pc):
    # RET
    if w == 0xD65F03C0:
        return "ret"
    # NOP
    if w == 0xD503201F:
        return "nop"
    # BL imm26
    if (w & 0xFC000000) == 0x94000000:
        imm = w & 0x3FFFFFF
        if imm & 0x2000000:
            imm -= 0x4000000
        return f"bl 0x{pc + imm * 4:x}"
    # B imm26
    if (w & 0xFC000000) == 0x14000000:
        imm = w & 0x3FFFFFF
        if imm & 0x2000000:
            imm -= 0x4000000
        return f"b 0x{pc + imm * 4:x}"
    # B.cond
    if (w & 0xFF000010) == 0x54000000:
        imm = (w >> 5) & 0x7FFFF
        if imm & 0x40000:
            imm -= 0x80000
        conds = ["eq","ne","cs","cc","mi","pl","vs","vc","hi","ls","ge","lt","gt","le","al","nv"]
        return f"b.{conds[w & 0xF]} 0x{pc + imm * 4:x}"
    # CBZ/CBNZ
    if (w & 0x7E000000) == 0x34000000:
        sf = (w >> 31) & 1
        op = (w >> 24) & 1
        imm = (w >> 5) & 0x7FFFF
        if imm & 0x40000:
            imm -= 0x80000
        name = "cbnz" if op else "cbz"
        return f"{name} {reg(w & 0x1F, sf)}, 0x{pc + imm * 4:x}"
    # TBZ/TBNZ
    if (w & 0x7E000000) == 0x36000000:
        op = (w >> 24) & 1
        b5 = (w >> 31) & 1
        b40 = (w >> 19) & 0x1F
        bit = (b5 << 5) | b40
        imm = (w >> 5) & 0x3FFF
        if imm & 0x2000:
            imm -= 0x4000
        name = "tbnz" if op else "tbz"
        return f"{name} {reg(w & 0x1F, 1)}, #{bit}, 0x{pc + imm * 4:x}"
    # ADRP
    if (w & 0x9F000000) == 0x90000000:
        immlo = (w >> 29) & 3
        immhi = (w >> 5) & 0x7FFFF
        imm = (immhi << 2) | immlo
        if imm & 0x100000:
            imm -= 0x200000
        page = (pc & ~0xFFF) + (imm << 12)
        return f"adrp x{w & 0x1F}, 0x{page & 0xFFFFFFFF:x}"
    # ADR
    if (w & 0x9F000000) == 0x10000000:
        immlo = (w >> 29) & 3
        immhi = (w >> 5) & 0x7FFFF
        imm = (immhi << 2) | immlo
        if imm & 0x100000:
            imm -= 0x200000
        return f"adr x{w & 0x1F}, 0x{(pc + imm) & 0xFFFFFFFF:x}"
    # MOVZ / MOVK / MOVN
    if (w & 0x1F800000) == 0x12800000:
        sf = (w >> 31) & 1
        opc = (w >> 29) & 3
        hw = (w >> 21) & 3
        imm = (w >> 5) & 0xFFFF
        rd = w & 0x1F
        names = {0: "movn", 2: "movz", 3: "movk"}
        name = names.get(opc, f"mov?{opc}")
        return f"{name} {reg(rd, sf)}, #0x{imm:x}{', lsl #' + str(hw*16) if hw else ''}"
    # ADD/SUB imm
    if (w & 0x1F000000) == 0x11000000:
        sf = (w >> 31) & 1
        op = (w >> 30) & 1
        S = (w >> 29) & 1
        sh = (w >> 22) & 1
        imm = (w >> 10) & 0xFFF
        rn = (w >> 5) & 0x1F
        rd = w & 0x1F
        if sh:
            imm <<= 12
        name = ("subs" if S else "sub") if op else ("adds" if S else "add")
        rn_s = "sp" if rn == 31 else reg(rn, sf)
        rd_s = "sp" if rd == 31 and not S else reg(rd, sf)
        return f"{name} {rd_s}, {rn_s}, #{imm}"
    # STR/LDR unsigned imm (store/load register, unsigned offset)
    # size opc 111 00 1 imm12 Rn Rt   — bit 24 distinguishes? 
    # LDR/STR unsigned: bits 29-24 = 111001, bit 22 is part of opc
    # Actually: 1x 111 0 01 imm12 for unsigned offset (bits 29-22)
    if (w & 0x3B000000) == 0x39000000:
        size = (w >> 30) & 3
        opc = (w >> 22) & 3
        imm12 = (w >> 10) & 0xFFF
        rn = (w >> 5) & 0x1F
        rt = w & 0x1F
        scale = size
        imm = imm12 << scale
        is_load = opc & 1
        sf = 1 if size == 3 else 0
        width = {0: "b", 1: "h", 2: "", 3: ""}[size]
        if size == 3:
            width = ""
            sf = 1
        name = ("ldr" if is_load else "str") + (width if size < 2 else "")
        if size == 2 and not is_load:
            name = "str"
        if size == 2 and is_load:
            name = "ldr"
        if size == 3 and not is_load:
            name = "str"
        if size == 3 and is_load:
            name = "ldr"
        base = "sp" if rn == 31 else f"x{rn}"
        return f"{name} {reg(rt, sf if size >= 2 else 0)}, [{base}, #{imm}]"
    # STP/LDP signed offset / pre / post
    if (w & 0x3E000000) == 0x28000000 or (w & 0x7C000000) == 0x28000000:
        opc = (w >> 30) & 3
        kind = (w >> 23) & 7
        if opc in (0, 2) and kind in (1, 2, 3):
            imm7 = (w >> 15) & 0x7F
            if imm7 & 0x40:
                imm7 -= 0x80
            scale = 2 if opc == 0 else 3
            imm = imm7 << scale
            rt2 = (w >> 10) & 0x1F
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            load = (w >> 22) & 1
            sf = 1 if opc == 2 else 0
            name = ("ldp" if load else "stp")
            base = "sp" if rn == 31 else f"x{rn}"
            mode = {1: "post", 3: "pre", 2: "off"}[kind]
            if mode == "off":
                return f"{name} {reg(rt, sf)}, {reg(rt2, sf)}, [{base}, #{imm}]"
            if mode == "pre":
                return f"{name} {reg(rt, sf)}, {reg(rt2, sf)}, [{base}, #{imm}]!"
            return f"{name} {reg(rt, sf)}, {reg(rt2, sf)}, [{base}], #{imm}"
    # LDR/STR unscaled / pre / post (bits 29-24 = 111000, bits 21-10 simm9)
    if (w & 0x3B200C00) == 0x38000000:
        size = (w >> 30) & 3
        opc = (w >> 22) & 3
        imm9 = (w >> 12) & 0x1FF
        if imm9 & 0x100:
            imm9 -= 0x200
        mode = (w >> 10) & 3
        rn = (w >> 5) & 0x1F
        rt = w & 0x1F
        sf = 1 if size == 3 else 0
        is_load = (opc & 1) == 1
        suf = {0: "b", 1: "h", 2: "", 3: ""}[size]
        name = ("ldr" if is_load else "str") + suf
        base = "sp" if rn == 31 else f"x{rn}"
        modes = {0: f"[{base}, #{imm9}]", 1: f"[{base}], #{imm9}", 3: f"[{base}, #{imm9}]!"}
        return f"{name} {reg(rt, sf if size >= 2 else 0)}, {modes.get(mode, '?')}"
    # Logical shifted register ORR/AND/EOR (includes MOV reg = ORR Rd, ZR, Rm)
    if (w & 0x1F000000) == 0x0A000000:
        sf = (w >> 31) & 1
        opc = (w >> 29) & 3
        n = (w >> 21) & 1  # not used for reg form? bit 21 is N for imm, for reg it's shift
        # register form: sf opc 01010 shift N Rm imm6 Rn Rd — bit 24-21 = 1010? 
        # Actually logical register: bits 28-24 = 01010
        pass
    if (w & 0x1F200000) == 0x0A000000:
        sf = (w >> 31) & 1
        opc = (w >> 29) & 3
        shift = (w >> 22) & 3
        n = (w >> 21) & 1
        rm = (w >> 16) & 0x1F
        imm6 = (w >> 10) & 0x3F
        rn = (w >> 5) & 0x1F
        rd = w & 0x1F
        names = {0: "and", 1: "orr", 2: "eor", 3: "ands"}
        name = names[opc]
        if n:
            name = {"and": "bic", "orr": "orn", "eor": "eon", "ands": "bics"}[name]
        if opc == 1 and n == 0 and rn == 31 and imm6 == 0 and shift == 0:
            return f"mov {reg(rd, sf)}, {reg(rm, sf)}"
        shn = ["lsl", "lsr", "asr", "ror"][shift]
        extra = f", {shn} #{imm6}" if imm6 else ""
        return f"{name} {reg(rd, sf)}, {reg(rn, sf)}, {reg(rm, sf)}{extra}"
    # ADD/SUB shifted register
    if (w & 0x1F200000) == 0x0B000000:
        sf = (w >> 31) & 1
        op = (w >> 30) & 1
        S = (w >> 29) & 1
        shift = (w >> 22) & 3
        rm = (w >> 16) & 0x1F
        imm6 = (w >> 10) & 0x3F
        rn = (w >> 5) & 0x1F
        rd = w & 0x1F
        name = ("subs" if S else "sub") if op else ("adds" if S else "add")
        shn = ["lsl", "lsr", "asr", "ror"][shift]
        extra = f", {shn} #{imm6}" if imm6 else ""
        return f"{name} {reg(rd, sf)}, {reg(rn, sf)}, {reg(rm, sf)}{extra}"
    # UBFM / LSL / LSR / SXTW aliases (bitfield)
    if (w & 0x1F800000) == 0x13000000:
        sf = (w >> 31) & 1
        opc = (w >> 29) & 3
        n = (w >> 22) & 1
        immr = (w >> 16) & 0x3F
        imms = (w >> 10) & 0x3F
        rn = (w >> 5) & 0x1F
        rd = w & 0x1F
        names = {0: "sbfm", 1: "bfm", 2: "ubfm"}
        return f"{names.get(opc,'bfm?')} {reg(rd, sf)}, {reg(rn, sf)}, #{immr}, #{imms}"
    # CMP/CMN imm is SUBS/ADDS with Rd=ZR, already handled.
    # CSEL / CSINC / CSINV / CSNEG
    if (w & 0x1FE00C00) == 0x1A800000:
        sf = (w >> 31) & 1
        op = (w >> 30) & 1
        rm = (w >> 16) & 0x1F
        cond = (w >> 12) & 0xF
        op2 = (w >> 10) & 3
        rn = (w >> 5) & 0x1F
        rd = w & 0x1F
        names = {(0, 0): "csel", (0, 1): "csinc", (1, 0): "csinv", (1, 1): "csneg"}
        name = names.get((op, op2), "cs?")
        conds = ["eq","ne","cs","cc","mi","pl","vs","vc","hi","ls","ge","lt","gt","le","al","nv"]
        return f"{name} {reg(rd, sf)}, {reg(rn, sf)}, {reg(rm, sf)}, {conds[cond]}"
    # MADD/MSUB
    if (w & 0x1F800000) == 0x1B000000:
        sf = (w >> 31) & 1
        op = (w >> 21) & 1  # not quite
        return f".madd 0x{w:08x}"
    return f".word 0x{w:08x}"


def main():
    data = SO.read_bytes()
    sections = parse_sections(data)
    off = va_off(sections, VA)
    blob = data[off:off + SIZE]
    print(f"va=0x{VA:x} file+0x{off:x} bytes={len(blob)}")
    for i in range(0, len(blob), 4):
        w = struct.unpack_from("<I", blob, i)[0]
        pc = VA + i
        print(f"  0x{pc:x}:  {w:08x}  {dis(w, pc)}")


if __name__ == "__main__":
    main()
