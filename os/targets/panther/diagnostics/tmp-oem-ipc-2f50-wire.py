#!/usr/bin/env python3
"""Dig libsitril 0x2f50 hits + OEM IPC wire header evidence in MAIN.

No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

RIL = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
BASE = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-base.so"
)
MAIN = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)

VA = 0x40010000
MAIN_OFF = 0x16C10
img = MAIN.read_bytes()
ril = RIL.read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def cstr(b, off, n=80):
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


# --- libsitril 0x2f50 contexts ---
log("=== libsitril u16 0x2f50 contexts ===")
needle = struct.pack("<H", 0x2F50)
pos = 0
hits = []
while True:
    i = ril.find(needle, pos)
    if i < 0:
        break
    hits.append(i)
    pos = i + 1

for i in hits:
    # show surrounding as hex + nearby ASCII
    lo = max(0, i - 32)
    hi = min(len(ril), i + 48)
    log(f"\n@{hex(i)}:")
    log(f"  hex: {dump(ril, lo, hi - lo)}")
    # scan backward for C-string
    for back in range(i, max(0, i - 0x200), -1):
        if ril[back] == 0 and back + 1 < i:
            s = cstr(ril, back + 1, 60)
            if s and len(s) >= 4:
                log(f"  prev_str@{hex(back+1)}: {s!r}")
                break
    # if this is part of a larger constant table of msgids
    nearby_ids = []
    for off in range(i - 32, i + 48, 2):
        if 0 <= off < len(ril) - 1:
            v = u16(ril, off)
            if 0x2F00 <= v <= 0x2FFF:
                nearby_ids.append((off, v))
    log(f"  nearby 0x2fxx: {[(hex(a), hex(b)) for a, b in nearby_ids[:20]]}")


log("\n=== libsitril 'SIM_INIT' context ===")
i = ril.find(b"SIM_INIT")
log(f"@{hex(i)}: {cstr(ril, i, 120)!r}")
# dump nearby for OEM/IPC hints
for key in (b"SIM_INIT", b"SIM_RESET", b"oem", b"OEM", b"Ipc", b"IPC"):
    pos = max(0, i - 0x200)
    end = min(len(ril), i + 0x200)
    chunk = ril[pos:end]
    # find strings in window
    p = 0
    while p < len(chunk):
        if 32 <= chunk[p] < 127:
            s = cstr(chunk, p, 70)
            if s and len(s) >= 6 and any(k.decode(errors="ignore").lower() in s.lower() for k in (b"sim", b"oem", b"ipc", b"init", b"reset")):
                log(f"  win_str@{hex(pos+p)}: {s!r}")
            p += len(s) if s else 1
        else:
            p += 1

# Symbol-ish: look for dynamic string SIM_INIT_REQ in all so
log("\n=== SO scan SIM_INIT_REQ / 2f50 / oem_ipc ===")
for path in (RIL, STREAM, BASE):
    data = path.read_bytes()
    log(f"\n-- {path.name} --")
    for key in (
        b"SIM_INIT_REQ",
        b"SIM_INIT or SIM_RESET",
        b"oem_ipc",
        b"/dev/oem",
        b"OEM_IPC",
        b"OemIpc",
        b"sipc",
        b"SIPC",
        b"fmt_hdr",
        b"IPC_HDR",
    ):
        n = data.count(key)
        if n:
            idx = data.find(key)
            log(f"  {key!r} x{n} eg {cstr(data, idx, 80)!r}")


# --- Samsung FMT/OEM header: search MAIN for known patterns ---
# Classic Samsung IPC FMT header is often: 7F | len_lo | len_hi | msg_seq | ...
# Or SIT we already know: type | pad | id_lo | id_hi | len_lo | len_hi | token...
log("\n=== MAIN: strings tying OEM IPC to host channel names ===")
for key in (
    b"oem_ipc",
    b"OEM_IPC",
    b"umts_ipc",
    b"[OEM][IPC] Sent IPC message",
    b"[OEM][IPC] Unable to send IPC message",
    b"Unable to encode IPC message",
    b"encoded size",
    b"Host interface is not ready",
    b"Sending data length",
    b"Received packet from channel",
):
    pos = 0
    n = 0
    while n < 5:
        i = img.find(key, pos)
        if i < 0:
            break
        log(f"  {hex(i)}: {cstr(img, i, 90)!r}")
        n += 1
        pos = i + 1


# Catalog table base: find pointer to first entry or array head near 0x6de000
log("\n=== Catalog neighborhood structure before SIM bank ===")
# walk backward from 0x6de60c looking for non-SIM entries / table header
o = 0x6DE60C - 28
for _ in range(12):
    flags = u16(img, o)
    msgid = u16(img, o + 2)
    name_va = u32(img, o + 4)
    name = cstr(img, MAIN_OFF + (name_va - VA), 50) if 0x40000000 <= name_va <= 0x42000000 else None
    meta = u32(img, o + 8)
    log(f"  @{hex(o)} flags={hex(flags)} id={hex(msgid)} meta={hex(meta)} name={name!r}")
    o -= 28


# Look for encode function: MOVW rN,#0x2f50 near OEM bank code that stores into buffer
# Scan code that does STRH of msgid after loading catalog - hard.
# Instead: find ADR of catalog base 0x406d7xxx via literal pools
log("\n=== Literal pools containing catalog VAs (0x406d7xxx) ===")
# SIM_INIT entry VA 0x406d7b30; catalog region roughly 0x406d79xx-0x406d7xxx
found = []
for off in range(0x600000, 0xA00000, 4):
    w = u32(img, off)
    if 0x406D7900 <= w <= 0x406D7C00:
        found.append((off, w))
        if len(found) >= 30:
            break
log(f"  lit hits in 0x6..0xa M: {len(found)}")
for off, w in found[:20]:
    log(f"  lit@{hex(off)} = {hex(w)}")


# Broader for any 0x406d7xxx
found2 = []
for off in range(0x600000, 0xC00000, 4):
    w = u32(img, off)
    if 0x406D6000 <= w <= 0x406D9000:
        found2.append((off, w))
        if len(found2) >= 40:
            break
log(f"  lit hits catalog-ish: {len(found2)}")
for off, w in found2[:25]:
    # nearby strings?
    name = None
    so = MAIN_OFF + (w - VA)
    # if points into entry name field
    log(f"  @{hex(off)} -> {hex(w)}")


# --- What is flags field? Compare abs sizes ---
# If flags = encoded body length, INIT body = 2 bytes. What are those 2 bytes?
# Look at USIM handler 0x43909d49 for expected fields (decode)
handler_va = 0x43909D49
# Thumb bit
handler_off = MAIN_OFF + ((handler_va & ~1) - VA)
log(f"\n=== SIM_INIT handler @{hex(handler_va)} off={hex(handler_off)} ===")
log(f"  bytes: {dump(img, handler_off, 0x80)}")
# Look for LDRB/LDRH size immediates / CMP for length
# Simple: find immediate compares 2, 4, 8 near start
from struct import unpack

# Disassemble lightly: find MOVW/CMP immediates in first 0x100
def movw_at(o):
    hw, hw2 = u16(img, o), u16(img, o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF

log("  MOVW in first 0x120:")
for o in range(handler_off, handler_off + 0x120, 2):
    r = movw_at(o)
    if r:
        log(f"    {hex(o)} imm={hex(r[0])} r{r[1]}")


# --- Check if 0x2f50 appears as SIT id in any sit builder tables in stream ---
stream = STREAM.read_bytes()
log("\n=== sit-stream: search builder id tables containing 0x2fxx ===")
# Protocol builders typically MOVW #id then STRH
# Already know no raw 0x2f50. Check 0x2f52 OEM verifypin too.
for mid in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x0201):
    n = stream.count(struct.pack("<H", mid))
    log(f"  u16 {hex(mid)} count={n}")

log("\nDONE")
