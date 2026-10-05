#!/usr/bin/env python3
import struct
from pathlib import Path

ss = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()


def dec(i, w):
    if w == 0xD65F03C0:
        return "RET"
    if (w & 0xFF800000) == 0x52800000:
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        rd = w & 0x1F
        return f"MOVZ w{rd},#{hex(imm16 << (hw * 16))}"
    if (w & 0xFFC00000) == 0xB9000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, ((w >> 10) & 0xFFF) * 4
        return f"STR w{rt},[x{rn},#{imm}]"
    if (w & 0xFFC00000) == 0x39000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, (w >> 10) & 0xFFF
        return f"STRB w{rt},[x{rn},#{imm}]"
    if (w & 0xFFC00000) == 0x79000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, ((w >> 10) & 0xFFF) * 2
        return f"STRH w{rt},[x{rn},#{imm}]"
    if (w & 0xFFC00000) == 0xB9400000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, ((w >> 10) & 0xFFF) * 4
        return f"LDR w{rt},[x{rn},#{imm}]"
    if (w & 0x7FE0FFE0) == 0x2A0003E0:
        rd, rm = w & 0x1F, (w >> 16) & 0x1F
        return f"MOV w{rd},w{rm}"
    if (w & 0xFC000000) == 0x94000000:
        imm26 = w & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        return f"BL {hex(i + imm26 * 4)}"
    if (w & 0x7E000000) == 0x34000000:
        rt = w & 0x1F
        imm19 = (w >> 5) & 0x7FFFF
        op = (w >> 24) & 1
        if imm19 & (1 << 18):
            imm19 -= 1 << 19
        tag = "CBNZ" if op else "CBZ"
        return f"{tag} w{rt},{hex(i + imm19 * 4)}"
    return f"?{hex(w)}"


print("=== 0x7f1d0..0x7f250 ===")
for i in range(0x7F1D0, 0x7F250, 4):
    w = struct.unpack_from("<I", ss, i)[0]
    print(hex(i), dec(i, w))
