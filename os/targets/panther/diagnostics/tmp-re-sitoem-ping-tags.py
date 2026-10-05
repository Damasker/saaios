#!/usr/bin/env python3
"""Recover SitOem IpcMessage protobuf field tags from carved .so."""
from __future__ import annotations

import struct
from pathlib import Path

SO = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/factory-td1a-vendor/carved-oemipc-25e7b000.so"
).read_bytes()


def u32(o):
    return struct.unpack_from("<I", SO, o)[0]


# Dump around PayloadCase string
i = SO.find(b"PayloadCase")
print("PayloadCase @", hex(i))
print(SO[i - 80 : i + 120])
# printable neighbors
ctx = SO[max(0, i - 200) : i + 300]
print("".join(chr(b) if 32 <= b < 127 else "." for b in ctx))

# Look for protobuf field number tables: often encoded as
# (number << 3) | wiretype in MOVZ near set_* 
# Or reflection: field numbers as small ints in descriptor

# Search mangled set_type / set_token / set_ping / mutable_ping
print("\n=== mangled set_/mutable_ near Ipc/Ping ===")
for n in (
    b"set_type",
    b"set_token",
    b"mutable_ping",
    b"set_ping",
    b"mutable_request",
    b"set_request",
    b"set_data",
    b"set_msg",
    b"has_ping",
    b"ping_",
    b"kPingFieldNumber",
    b"kTypeFieldNumber",
    b"kTokenFieldNumber",
    b"TYPE_FIELD_NUMBER",
    b"TOKEN_FIELD_NUMBER",
    b"PING_FIELD_NUMBER",
):
    hits = []
    start = 0
    while True:
        j = SO.find(n, start)
        if j < 0:
            break
        hits.append(j)
        start = j + 1
        if len(hits) >= 8:
            break
    if hits:
        print(f"  {n.decode()}: {[hex(h) for h in hits]}")
        for h in hits[:2]:
            print("   ", "".join(chr(b) if 32 <= b < 127 else "." for b in SO[h : h + 80]))

# Google protobuf generated code often has:
# static const int kXxxFieldNumber = N;
# as .rodata 32-bit little-endian near name string — or in MOVZ in setter.

# Disassemble functions that contain "set_type" string refs via ADRP+ADD
# Simpler: find all MOVZ #1..#20 in .text near Serialize patterns

# Extract FileDescriptorProto-like: look for ".sit_ipc_message.IpcMessage"
for n in (
    b"sit_ipc_message.IpcMessage",
    b"sit_ipc_message.PingMessage",
    b"sit_ipc_message.PingRequest",
    b"IpcMessageType",
    b"ipc_message.proto",
    b"ping.proto",
    b"oem_ipc.proto",
    b"type\x00",
    b"token\x00",
):
    j = SO.find(n)
    print(f"{n!r}: {hex(j) if j>=0 else None}")
    if j is not None and j >= 0:
        print(" ", "".join(chr(b) if 32 <= b < 127 else "." for b in SO[max(0,j-40):j+80]))

# Scan .rodata for protobuf FieldDescriptorProto number sequences near "type"/"token"/"ping"
# Classic: field name string followed soon by field number byte in descriptor pool
print("\n=== search descriptor pool blobs for field names type/token/ping ===")
rod_off = 0xBF90
rod = SO[rod_off : rod_off + 0x1134]
for name in (b"type", b"token", b"ping", b"request", b"response", b"data", b"msg"):
    idx = 0
    while True:
        j = rod.find(name + b"\x00", idx)
        if j < 0:
            break
        abs_off = rod_off + j
        # show surrounding 32 bytes as hex+ascii
        chunk = SO[abs_off : abs_off + 48]
        print(f"  @{hex(abs_off)} {name.decode()} :: {chunk.hex()} | {''.join(chr(b) if 32<=b<127 else '.' for b in chunk)}")
        idx = j + 1

# initialMessageHeader: args are (PayloadCase, IpcMessageType)
# Typically sets type_ then clears payload then sets case
# Find callee that sets type — from initialMessageHeader BL 0x1e5b0
print("\n=== initialMessageHeader body (raw) ===")
off = 0x1AA70
for i in range(0, 0x80, 4):
    w = u32(off + i)
    # MOVZ
    if (w & 0xFF800000) == 0x52800000:
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        rd = w & 0x1F
        print(f"  @{hex(off+i)} MOVZ w{rd},#{hex(imm16<<(hw*16))}")
    elif (w & 0xFFC00000) == 0x39000000:
        imm = (w >> 10) & 0xFFF
        print(f"  @{hex(off+i)} STRB #{imm}")
    elif (w & 0xFFC00000) == 0xB9000000:
        imm = ((w >> 10) & 0xFFF) * 4
        print(f"  @{hex(off+i)} STR #{imm}")
    elif (w & 0xFC000000) == 0x94000000:
        imm26 = w & 0x03FFFFFF
        if imm26 & 0x2000000:
            imm26 -= 0x4000000
        print(f"  @{hex(off+i)} BL {hex(off+i+imm26*4)}")
    elif (w & 0xFFFFFC1F) == 0xD65F0000:
        print(f"  @{hex(off+i)} RET")
        break

# Look at PLT stub resolution - can't easily. Instead search for
# protobuf wire encoding helper that writes tag = (field<<3)|wt
# Pattern: MOVZ wid, #(field<<3)|wt  common tags: type=1 -> 0x08, token=2 -> 0x10, ping=5 -> 0x2a
print("\n=== MOVZ of classic protobuf tags in .text ===")
tags = {0x08: "f1_varint", 0x10: "f2_varint", 0x1A: "f3_len", 0x22: "f4_len", 0x2A: "f5_len", 0x0A: "f1_len", 0x12: "f2_len"}
counts = {k: [] for k in tags}
for i in range(0x10000, 0x10000 + 0xD5AC, 4):
    w = u32(i)
    if (w & 0xFF800000) == 0x52800000:
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        if hw == 0 and imm16 in tags:
            counts[imm16].append(i)
for k, v in counts.items():
    print(f"  MOVZ #{hex(k)} ({tags[k]}): n={len(v)} first={[hex(x) for x in v[:6]]}")

# Ping fillInput more carefully - full instruction dump including missed ops
print("\n=== fillInput full words ===")
off = 0x1B6D0
for i in range(0, 0xC0, 4):
    w = u32(off + i)
    print(f"  @{hex(off+i)} {w:08x}")
    if (w & 0xFFFFFC1F) == 0xD65F0000:
        # may be early RET on error path; continue a bit
        if i > 0x40:
            break

print("DONE")
