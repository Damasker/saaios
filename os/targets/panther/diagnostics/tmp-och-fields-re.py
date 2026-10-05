#!/usr/bin/env python3
import struct
from pathlib import Path

ss = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
ril = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
).read_bytes()

print("=== TransmitApduChannel stores ===")
for i in range(0x7F140, 0x7F2A0, 4):
    w = struct.unpack_from("<I", ss, i)[0]
    s = None
    if (w & 0xFFC00000) == 0xB9000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, ((w >> 10) & 0xFFF) * 4
        s = f"STR w{rt},[x{rn},#{imm}]"
    elif (w & 0xFFC00000) == 0x39000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, (w >> 10) & 0xFFF
        s = f"STRB w{rt},[x{rn},#{imm}]"
    elif (w & 0xFFC00000) == 0x79000000:
        rt, rn, imm = w & 0x1F, (w >> 5) & 0x1F, ((w >> 10) & 0xFFF) * 2
        s = f"STRH w{rt},[x{rn},#{imm}]"
    elif (w & 0xFF800000) == 0x52800000:
        imm16, hw, rd = (w >> 5) & 0xFFFF, (w >> 21) & 3, w & 0x1F
        s = f"MOVZ w{rd},#{hex(imm16 << (hw * 16))}"
    elif (w & 0xFC000000) == 0x94000000:
        imm26 = w & 0x3FFFFFF
        if imm26 & (1 << 25):
            imm26 -= 1 << 26
        s = f"BL {hex(i + imm26 * 4)}"
    if s:
        print(hex(i), s)

print("\n=== OpenChannel GetSessionID / GetSw ===")
for name, va in [("GetSessionID", 0x67160), ("GetSw1", 0x671A0), ("GetSw2", 0x671E0)]:
    print(f"-- {name} @{hex(va)}")
    for i in range(va, va + 0x40, 4):
        w = struct.unpack_from("<I", ss, i)[0]
        print(hex(i), hex(w))

print("\n=== OnVerifyPinDone interesting BLs ===")
# OnVerifyPinDone @ 0x1a5e50 in libsitril
va = 0x1A5E50
for i in range(0, 0x280, 4):
    insn = struct.unpack_from("<I", ril, va + i)[0]
    if (insn & 0xFC000000) != 0x94000000:
        continue
    imm26 = insn & 0x03FFFFFF
    if imm26 & 0x02000000:
        imm26 -= 0x04000000
    tgt = va + i + (imm26 << 2)
    print(f"BL @{hex(va+i)} -> {hex(tgt)}")
