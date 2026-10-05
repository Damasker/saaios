#!/usr/bin/env python3
"""Deeper GetPinState / GetAppState offset proof + SitOem Ping PayloadCase."""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
SS = (DIAG / "sit-stream.so").read_bytes()
LS = (DIAG / "libsitril.so").read_bytes()
SO = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
).read_bytes()


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def dump_func(blob, off, n=120, label=""):
    print(f"\n=== {label} off={hex(off)} ===")
    for i in range(0, n * 4, 4):
        w = u32(blob, off + i)
        va = off + i  # file VA==off for these ELFs
        # decode common
        if (w & 0xFFC00000) == 0x39400000:
            imm = (w >> 10) & 0xFFF
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            print(f"  @{hex(va)} LDRB w{rt},[x{rn},#{imm}]")
        elif (w & 0xFFC00000) == 0x39000000:
            imm = (w >> 10) & 0xFFF
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            print(f"  @{hex(va)} STRB w{rt},[x{rn},#{imm}]")
        elif (w & 0xFFC00000) == 0x79400000:
            imm = ((w >> 10) & 0xFFF) * 2
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            print(f"  @{hex(va)} LDRH w{rt},[x{rn},#{imm}]")
        elif (w & 0xFFC00000) == 0xB9400000:
            imm = ((w >> 10) & 0xFFF) * 4
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            print(f"  @{hex(va)} LDR w{rt},[x{rn},#{imm}]")
        elif (w & 0xFF000000) == 0x91000000:
            sh = (w >> 22) & 1
            imm = ((w >> 10) & 0xFFF) << (12 if sh else 0)
            rn = (w >> 5) & 0x1F
            rd = w & 0x1F
            print(f"  @{hex(va)} ADD x{rd},x{rn},#{imm}")
        elif (w & 0xFF800000) == 0x52800000:
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            print(f"  @{hex(va)} MOVZ w{rd},#{hex(imm16 << (hw * 16))}")
        elif (w & 0xFF800000) == 0x12B00000 or (w & 0xFF800000) == 0x72A00000:
            # MOVK
            imm16 = (w >> 5) & 0xFFFF
            hw = (w >> 21) & 3
            rd = w & 0x1F
            print(f"  @{hex(va)} MOVK w{rd},#{hex(imm16)} LSL#{hw*16}")
        elif (w & 0x7E000000) == 0x34000000:
            # CBZ/CBNZ roughly
            rt = w & 0x1F
            imm19 = (w >> 5) & 0x7FFFF
            if imm19 & 0x40000:
                imm19 -= 0x80000
            op = "CBZ" if (w & 0x01000000) == 0 else "CBNZ"
            print(f"  @{hex(va)} {op} x{rt}, {hex(va + imm19 * 4)}")
        elif (w & 0xFF00001F) == 0x7100001F or (w & 0xFF000000) == 0x71000000:
            # SUBS/CMP imm
            imm = (w >> 10) & 0xFFF
            rn = (w >> 5) & 0x1F
            rd = w & 0x1F
            if rd == 31:
                print(f"  @{hex(va)} CMP x{rn},#{imm}")
            else:
                print(f"  @{hex(va)} SUBS x{rd},x{rn},#{imm}")
        elif (w & 0xFC000000) == 0x94000000:
            imm26 = w & 0x03FFFFFF
            if imm26 & 0x2000000:
                imm26 -= 0x4000000
            print(f"  @{hex(va)} BL {hex(va + imm26 * 4)}")
        elif (w & 0xFFFFFC1F) == 0xD65F0000:
            print(f"  @{hex(va)} RET")
            break
        elif w == 0xD503201F:
            print(f"  @{hex(va)} NOP")


# GetPinState @0x66a20 — full dump
dump_func(SS, 0x66A20, 80, "GetPinState")
dump_func(SS, 0x66A80, 60, "GetPinRemainCount")
dump_func(SS, 0x666A0, 80, "ProtocolSimStatusAdapter::Init")

# Hunt GetAppState on ProtocolSimStatusAdapter — may be inline in FillRil
# Search for LDRB #33 (adapter+33 = state) and #31 (type)
print("\n=== sit-stream LDRB #31/#33/#0x58/#0x59/#0x5A ===")
for want in (31, 33, 0x58, 0x59, 0x5A, 30, 51, 19, 21):
    hits = []
    for i in range(0, len(SS) - 4, 4):
        w = u32(SS, i)
        if (w & 0xFFC00000) == 0x39400000 and ((w >> 10) & 0xFFF) == want:
            hits.append(i)
    print(f"  LDRB #{want}: n={len(hits)} {[hex(h) for h in hits[:10]]}")

# BuildRilCardStatusApplications in libsitril @0x154f00
dump_func(LS, 0x154F00, 100, "BuildRilCardStatusApplications")

# FillRilCardStatusFromAdapter @0x1553e0 — look for app state store
dump_func(LS, 0x1553E0, 100, "FillRilCardStatusFromAdapter")

# covertAppStateToString @0x171c90 — enum string table evidence
dump_func(LS, 0x171C90, 50, "covertAppStateToString")

# SitOem: Ping ctor calls initialMessageHeader with PayloadCase
# From earlier: MOVZ w0,#0x18 then BL; MOVZ w1,#0x5
# sitSendPingReq: MOVZ w1,#0x1 before BL — type REQUEST=1
dump_func(SO, 0x1B590, 50, "PingMessageModemData ctor")
dump_func(SO, 0x1B6D0, 60, "Ping fillInput")
dump_func(SO, 0x15650, 80, "sitSendPingReq")

# Find SerializeToArray / ByteSize in SitOem via plt stubs near ping
print("\n=== SitOem strings mentioning serialize / ByteSize / type enum ===")
for n in (
    b"SerializeToArray",
    b"SerializeToString",
    b"ByteSize",
    b"ParseFromArray",
    b"REQUEST",
    b"RESPONSE",
    b"INDICATION",
    b"kPing",
    b"ping =",
    b"PayloadCase",
    b"payload_case",
):
    i = SO.find(n)
    print(f"  {n.decode(errors='replace')}: {hex(i) if i>=0 else None}")

# Look at .data.rel.ro for protobuf schemas / tables near Ping vtable 0x1f1d8
print("\n=== Ping vtable @0x1f1d8 ===")
vt = SO[0x1F1D8 : 0x1F1D8 + 64]
print(vt.hex())
for i in range(0, 64, 8):
    ptr = struct.unpack_from("<Q", vt, i)[0]
    print(f"  [{i}] {hex(ptr)}")

# Search protobuf wire tags used by set_allocated / set_string in fillInput callees
# BL targets from fillInput: 0x1e230, 0x1e680, 0x1e690, 0x1e6a0
for tgt, lab in (
    (0x1E230, "fillInput BL#1"),
    (0x1E680, "fillInput BL#2 set?"),
    (0x1E690, "fillInput BL#3"),
    (0x1E6A0, "fillInput BL#4"),
    (0x1D820, "ctor BL alloc?"),
    (0x1E200, "ctor BL header?"),
):
    dump_func(SO, tgt, 40, lab)

print("\nDONE")
