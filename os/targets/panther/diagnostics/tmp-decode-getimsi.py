#!/usr/bin/env python3
import struct
from pathlib import Path

sit = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
ril = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
).read_bytes()

addr = 0x7F2A0
print("=== BuildGetImsi dump ===")
for o in range(addr, addr + 0xC0, 4):
    w = struct.unpack_from("<I", sit, o)[0]
    extra = ""
    if (w & 0xFF800000) == 0x52800000:
        imm = (w >> 5) & 0xFFFF
        rd = w & 0x1F
        extra = f" MOVZ W{rd},#{hex(imm)} ({imm})"
    if (w & 0xFF800000) == 0xD2800000:
        imm = (w >> 5) & 0xFFFF
        rd = w & 0x1F
        extra = f" MOVZ X{rd},#{hex(imm)}"
    if (w & 0xFFC00000) == 0x39000000:
        rt = w & 0x1F
        rn = (w >> 5) & 0x1F
        imm = (w >> 10) & 0xFFF
        extra = f" STRB W{rt},[X{rn},#{imm}]"
    if (w & 0xFFC00000) == 0x79000000:
        rt = w & 0x1F
        rn = (w >> 5) & 0x1F
        imm = ((w >> 10) & 0xFFF) << 1
        extra = f" STRH W{rt},[X{rn},#{imm}]"
    if (w & 0xFFC00000) == 0xB9000000:
        rt = w & 0x1F
        rn = (w >> 5) & 0x1F
        imm = ((w >> 10) & 0xFFF) << 2
        extra = f" STR W{rt},[X{rn},#{imm}]"
    if (w & 0xFC000000) == 0x94000000:
        imm = w & 0x3FFFFFF
        if imm & (1 << 25):
            imm -= 1 << 26
        tgt = o + imm * 4
        extra = f" BL->{hex(tgt)}"
    if (w & 0xFFFFFC1F) == 0xD65F0000:
        extra = " RET"
    print(f" {hex(o)}: {w:08x}{extra}")

# Compare known GetStatus @0x7dbb0
print("\n=== BuildSimGetStatus id/len pattern (reference) ===")
for o in range(0x7DBB0, 0x7DBB0 + 0x60, 4):
    w = struct.unpack_from("<I", sit, o)[0]
    if (w & 0xFF800000) == 0x52800000:
        imm = (w >> 5) & 0xFFFF
        rd = w & 0x1F
        print(f" {hex(o)} MOVZ W{rd},#{hex(imm)} ({imm})")

for n in [
    b"DoGetImsi",
    b"GetImsi",
    b"BuildGetImsi",
    b"GET_IMSI",
    b"RIL_REQUEST_GET_IMSI",
    b"OnGetImsiDone",
]:
    print(n, hex(ril.find(n)) if ril.find(n) >= 0 else None)

# Also OemSimRequest and GetSimLockInfo
print("\nOemSim / LockInfo:")
for addr, lab in [(0x806D0, "BuildOemSimRequest"), (0x807C0, "BuildGetSimLockInfo")]:
    print(f"--- {lab} ---")
    for o in range(addr, addr + 0x80, 4):
        w = struct.unpack_from("<I", sit, o)[0]
        if (w & 0xFF800000) == 0x52800000:
            imm = (w >> 5) & 0xFFFF
            rd = w & 0x1F
            print(f" {hex(o)} MOVZ W{rd},#{hex(imm)} ({imm})")
        if (w & 0xFFFFFC1F) == 0xD65F0000:
            print(f" {hex(o)} RET")
            break
