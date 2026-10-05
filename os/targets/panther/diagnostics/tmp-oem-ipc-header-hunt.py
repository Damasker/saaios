#!/usr/bin/env python3
"""Hunt OEM IPC host wire header near ipc_message_server / catalog decode.

No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA = 0x40010000
MAIN = 0x16C10


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va_of(o):
    return VA + (o - MAIN)


def cstr(o, n=100):
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def dump(o, n=64):
    return " ".join(f"{x:02x}" for x in img[o : o + n])


# 1) All strings under ipc_message_server path
log("=== ipc_message_server source/path strings ===")
pos = 0
n = 0
while n < 60:
    i = img.find(b"ipc_message_server", pos)
    if i < 0:
        break
    # back up to path start
    s = cstr(max(0, i - 80), 160)
    log(f"  {hex(i)}: {s!r}")
    n += 1
    pos = i + 1

# 2) Nearby encode/decode error strings already known — find format of header fields
log("\n=== OEM IPC field / header strings ===")
keys = [
    b"message_id",
    b"Message ID",
    b"msg_id",
    b"MsgId",
    b"msgid",
    b"MSGID",
    b"ipc_len",
    b"header_len",
    b"hdr_len",
    b"magic",
    b"0x7F",
    b"SEQ",
    b"seq_num",
    b"transaction",
    b"txn_id",
    b"Request message",
    b"not a REQUEST",
    b"preprocess",
    b"encoded size",
    b"Unable to encode",
    b"Unable to decode",
    b"Invalid message",
    b"SIM_INIT_REQ",
    b"SIM_INFO_REQ",
    b"SIM_VERIFYPIN_REQ",
]
for k in keys:
    pos = 0
    hits = 0
    while hits < 3:
        i = img.find(k, pos)
        if i < 0:
            break
        # filter to OEM-ish region or general
        s = cstr(i, 90)
        if s and ("OEM" in s or "IPC" in s or "SIM_" in s or "encode" in s.lower() or "decode" in s.lower() or "message" in s.lower() or "Request" in s or "preprocess" in s):
            log(f"  {hex(i)}: {s!r}")
            hits += 1
        elif k in (b"SIM_INIT_REQ", b"SIM_INFO_REQ", b"SIM_VERIFYPIN_REQ", b"magic", b"0x7F"):
            log(f"  {hex(i)}: {s!r}")
            hits += 1
        pos = i + 1

# 3) Catalog entry: flags field usage — find CMP to flags-like loads from [entry,#0]
# Look for code that does LDRH from a pointer then compares msgid 0x2f50 — consumer decode
log("\n=== Bare MOVW #0x2f50 in OEM band 0x6d0000..0x700000 (code?) ===")
# OEM strings at 0x6dxxxx are rodata; code for OEM may be elsewhere.
# Search whole image for MOVW #0x2f50 followed within 0x20 by something store-ish

def movw(o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def movt(o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF2C0 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


# Find movw #0x2f50 WITHOUT following movt same reg (bare msgid)
bare = []
o = 0x1000000
while o < 0x3A00000 and len(bare) < 30:
    r = movw(o)
    if r and r[0] == 0x2F50:
        # check no movt same reg in next 8 instr
        has_movt = False
        p = o + 4
        for _ in range(8):
            t = movt(p)
            if t and t[1] == r[1]:
                has_movt = True
                break
            hw = u16(p)
            p += 4 if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) else 2
            if p >= len(img):
                break
        if not has_movt:
            bare.append(o)
    o += 2
log(f"bare MOVW #0x2f50 count={len(bare)}")
for o in bare[:15]:
    log(f"  {hex(o)}: {dump(o, 0x30)}")

# 4) meta 0x10104 meaning: search MOVW #0x0104 or #0x10104 usage
log("\n=== MOVW related to meta pattern 0x0104 / 0x0304 ===")
for imm in (0x0104, 0x0304, 0x101, 0x103):
    hits = []
    o = 0x600000
    while o < 0x800000 and len(hits) < 5:
        r = movw(o)
        if r and r[0] == imm:
            hits.append(o)
        o += 2
    log(f"  MOVW #{hex(imm)} in 0x6..0x8M: {[hex(x) for x in hits]}")

# 5) Compare catalog flags to known soft SIT payload sizes where dual exists
# POWER_CHANGE OEM flags=1; soft CardPower SIT 0x024c has 1-byte state payload after hdr?
log("\n=== Catalog flags vs soft SIT known sizes (parallel ids, NOT proof of dual) ===")
log("  OEM INIT flags=2; soft has NO SIT dual")
log("  OEM VERIFYPIN flags=0xa(10); soft SIT 0x0201 total len=38 (hdr12+body26) — NOT equal")
log("  => flags is OEM-encoded-body size, not SIT total length")

# 6) Is there a host-facing 'SIT OEM' channel that wraps OEM msgid inside SIT?
# Search sit-stream for string OEM
stream = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
log("\n=== sit-stream OEM/IPC channel strings ===")
for k in (b"OEM", b"oem", b"Ipc", b"IPC", b"channel", b"2f50", b"SIM_INIT"):
    c = stream.count(k)
    if c:
        i = stream.find(k)
        s = "".join(chr(x) if 32 <= x < 127 else "." for x in stream[i : i + 60])
        log(f"  {k!r} x{c} eg {s!r}")

# 7) Final verdict fields
log("\n=== VERDICT INPUTS ===")
log("catalog SIM_INIT @0x6de740: flags=2 msgid=0x2f50 meta=0x10104 rsp_id=0 stride=28")
log("soft VerifyPin transport: umts_ipc0 SIT type0 id=0x0201 len=38 — NOT OEM 0x2f52")
log("sit-stream Build*SimInit: ABSENT; u16 0x2f50 count=0")
log("oem_ipc* chardev: exists (mknod); prior RO listen EOF; write framing UNKNOWN")
log("libsitril MOVZ #0x2f50: table-init only, no packet builder")
log("SIT↔OEM msgid dual mapping in MAIN: NOT found for 0x200/0x2f50 or 0x201/0x2f52")

log("\nDONE")
