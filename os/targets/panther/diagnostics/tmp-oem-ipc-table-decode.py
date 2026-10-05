#!/usr/bin/env python3
"""Decode libsitril OEM msgid table @0x95660 + sit-stream 0x2f52 + OEM utils.

No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

RIL = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
).read_bytes()
STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
).read_bytes()
img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()

VA = 0x40010000
MAIN = 0x16C10


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def cstr(b, off, n=100):
    if off < 0 or off >= len(b):
        return None
    s = bytearray()
    for i in range(off, min(len(b), off + n)):
        c = b[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def dump(b, o, n=64):
    return " ".join(f"{x:02x}" for x in b[o : o + n])


# --- Walk libsitril table around 0x95648 ---
log("=== libsitril OEM-ish msgid table walk ===")
# Observed record ~0x18 bytes: msgid:u16, unk:u16, pad:u32, a:u32, b:u32, pad?
base = 0x95600
# find start of contiguous 0x2fxx entries
for start in range(0x95000, 0x96000, 4):
    if 0x2F00 <= u16(RIL, start) <= 0x2FFF and u16(RIL, start + 2) in (0x23, 0x20, 0x21, 0x22, 0x24):
        base = start
        break
log(f"candidate base@{hex(base)}")

# Try stride 0x18
o = base
for i in range(40):
    msgid = u16(RIL, o)
    unk = u16(RIL, o + 2)
    w2 = u32(RIL, o + 4)
    w3 = u32(RIL, o + 8)
    w4 = u32(RIL, o + 12)
    w5 = u32(RIL, o + 16)
    if not (0x2F00 <= msgid <= 0x3000 or 0x200 <= msgid <= 0x300):
        # maybe end
        if i > 5:
            log(f"  stop@{hex(o)} msgid={hex(msgid)}")
            break
    mark = ""
    if msgid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x201):
        mark = " <<<"
    log(
        f"  @{hex(o)} id={hex(msgid)} unk={hex(unk)} w2={hex(w2)} w3={hex(w3)} w4={hex(w4)} w5={hex(w5)}{mark}"
    )
    o += 0x18

# Also dump wider raw around 0x95660
log("\n=== raw 0x95580..0x95780 ===")
log(dump(RIL, 0x95580, 0x200))

# Find xrefs: who loads 0x95660? In ELF, need dynsym — approximate via ADRP+ADD patterns hard.
# Search for string tables near this for names
log("\n=== ASCII near 0x95660 ±2K ===")
pos = 0x95000
while pos < 0x96000:
    if 32 <= RIL[pos] < 127:
        s = cstr(RIL, pos, 70)
        if s and len(s) >= 5:
            log(f"  {hex(pos)}: {s!r}")
            pos += len(s)
        else:
            pos += 1
    else:
        pos += 1


# --- sit-stream 0x2f52 hits ---
log("\n=== sit-stream u16 0x2f52 contexts ===")
needle = struct.pack("<H", 0x2F52)
pos = 0
while True:
    i = STREAM.find(needle, pos)
    if i < 0:
        break
    # ARM64? or data table?
    # Check if it's in BL: bits of 32-bit insn
    insn = u32(STREAM, i & ~3) if i + 4 <= len(STREAM) else 0
    log(f"\n@{hex(i)} aligned32={hex(i&~3)} insn32={hex(insn)}")
    log(f"  hex: {dump(STREAM, max(0,i-16), 48)}")
    nearby = []
    for off in range(i - 24, i + 40, 2):
        if 0 <= off < len(STREAM) - 1:
            v = u16(STREAM, off)
            if 0x2F00 <= v <= 0x2FFF or 0x200 <= v <= 0x220:
                nearby.append((hex(off), hex(v)))
    log(f"  nearby ids: {nearby}")
    pos = i + 1

log("\n=== sit-stream u16 0x2f57 / 0x2f58 / 0x2f50 ===")
for mid in (0x2F50, 0x2F57, 0x2F58):
    pos = 0
    n = 0
    while n < 5:
        i = STREAM.find(struct.pack("<H", mid), pos)
        if i < 0:
            break
        log(f"  {hex(mid)}@{hex(i)}: {dump(STREAM, max(0,i-12), 40)}")
        n += 1
        pos = i + 1


# --- OEM utils source names → find encode format strings ---
log("\n=== MAIN oem_ipc_message_* string contexts ===")
for key in (
    b"oem_ipc_message_dispatcher.c",
    b"oem_ipc_message_utils.c",
    b"oem_ipc_message",
):
    pos = 0
    while True:
        i = img.find(key, pos)
        if i < 0:
            break
        # show surrounding strings in ±0x200
        log(f"\n{key.decode()} @{hex(i)}")
        window = img[max(0, i - 0x100) : i + 0x200]
        p = 0
        shown = 0
        while p < len(window) and shown < 25:
            if 32 <= window[p] < 127:
                s = cstr(window, p, 80)
                if s and len(s) >= 4:
                    log(f"  {s!r}")
                    shown += 1
                    p += len(s)
                else:
                    p += 1
            else:
                p += 1
        pos = i + 1


# Search encode-related format strings
log("\n=== MAIN encode/header-ish OEM strings ===")
for key in (
    b"IPC header",
    b"ipc header",
    b"msg_len",
    b"msg length",
    b"message length",
    b"payload length",
    b"encode_ipc",
    b"EncodeIpc",
    b"ipc_encode",
    b"OEM IPC",
    b"oem ipc",
    b"sipc_hdr",
    b"SIPC",
    b"fmt_msg",
    b"FMT_MSG",
    b"IPC_MSG",
    b"msg_seq",
    b"cmd_type",
):
    i = img.find(key)
    if i >= 0:
        log(f"  {hex(i)}: {cstr(img, i, 90)!r}")


# --- Does CP map OEM msgid 0x2f50 onto host SIT somehow? ---
# Search for pairs 0x0201 and 0x2f52 near each other (VerifyPin dual)
log("\n=== MAIN: 0x0201 near 0x2f52 (SIT↔OEM dual?) ===")
# scan for MOVW #0x201 and nearby #0x2f52 within 0x40
def movw(o):
    if o + 4 > len(img):
        return None
    hw, hw2 = u16(img, o), u16(img, o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

pairs = []
o = 0x600000
while o < 0xC00000 and len(pairs) < 20:
    r = movw(o)
    if r and r[0] == 0x201:
        for p in range(o - 0x40, o + 0x40, 2):
            r2 = movw(p)
            if r2 and r2[0] == 0x2F52:
                pairs.append((o, p))
                break
    o += 2
log(f"  MOVW 0x201 near 0x2f52 pairs: {[(hex(a),hex(b)) for a,b in pairs]}")

pairs2 = []
o = 0x600000
while o < 0xC00000 and len(pairs2) < 20:
    r = movw(o)
    if r and r[0] == 0x200:
        for p in range(o - 0x40, o + 0x40, 2):
            r2 = movw(p)
            if r2 and r2[0] == 0x2F50:
                pairs2.append((o, p))
                break
    o += 2
log(f"  MOVW 0x200 near 0x2f50 pairs: {[(hex(a),hex(b)) for a,b in pairs2]}")


# libsitril: does anything *send* 0x2f50? Search for MOVZ #0x2f50 in ARM64
# MOVZ Wd, #imm16: 0x52800000 | (imm<<5) | Rd
log("\n=== libsitril ARM64 MOVZ #0x2f50 / #0x2f52 ===")
for imm in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x201):
    # MOVZ Wd,#imm
    hits = []
    for rd in range(32):
        insn = 0x52800000 | (imm << 5) | rd
        needle = struct.pack("<I", insn)
        pos = 0
        while len(hits) < 8:
            i = RIL.find(needle, pos)
            if i < 0:
                break
            hits.append((i, rd))
            pos = i + 1
    log(f"  MOVZ #{hex(imm)} hits={len(hits)} {[(hex(i), f'w{rd}') for i,rd in hits[:6]]}")

log("\nDONE")
