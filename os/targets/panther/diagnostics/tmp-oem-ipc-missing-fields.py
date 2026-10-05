#!/usr/bin/env python3
"""Last-pass: OEM app header candidates from MAIN encode strings + catalog meta.

Only report fields with RE evidence. Do not propose a full wire frame.
"""
from __future__ import annotations

import struct
from pathlib import Path

img = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
).read_bytes()
VA0 = 0x40010000
MAIN = 0x16C10


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va_of(o):
    return VA0 + (o - MAIN)


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


# Catalog decode evidence
log("=== Catalog field meaning (evidence-limited) ===")
log("stride=28 evidenced by consecutive SIM_* ids")
log(" +0 u16 flags: INIT=2 INFO=2 STOP=2 VERIFYPIN=0xa — body-size hint (prior)")
log(" +2 u16 msgid")
log(" +4 u32 name_va")
log(" +8 u32 meta: REQ bank mostly 0x00010104; IND-ish 0x00010304")
log(" +0c u16/u32 rsp_id: INIT=0 (no paired rsp in table); VERIFYPIN=0x2fa1; INFO=0x2fa8")
log(" +18 u32: INIT=4 STOP=4 — unknown (not hdr len proof)")

# meta bit split
for label, meta in [("REQ", 0x10104), ("IND", 0x10304), ("GRR", 0x1030A), ("NS", 0x1031E)]:
    log(f"meta {label}={hex(meta)} bytes LE={[hex(meta&0xff), hex((meta>>8)&0xff), hex((meta>>16)&0xff), hex((meta>>24)&0xff)]}")

# Compare SIT host header (evidenced) vs what OEM would need
log("\n=== Evidenced transports ===")
log("1) umts_ipc0 SIT FMT (soft): type:u8 pad:u8 id:u16le len:u16le token:u32le [body]")
log("   VerifyPin id=0x0201 total_len=38 — NOT OEM 0x2f52")
log("2) CPIF PROTOCOL_SIT write path: kernel prepends EXYNOS 12B (sync=0xABCD,...) when iod->link_header")
log("   userspace must write APP payload only (same as umts_ipc0 lesson)")
log("3) cbd: /dev/umts_boot0|/dev/umts_ramdump0 only — no OEM encode")
log("4) libsitril: msgid registry stride24 id|0x23|0|0x402|idx — table-init MOVZ only; no /dev/oem_ipc; no builder")
log("5) sit-stream/sit-base: no oem_ipc string; no 0x2f50 builder")

# Search for message_id == kMessageId pattern — C++ typed IPC
s = img.find(b"message_id == kMessageId")
log(f"\nkMessageId assert str @{hex(s) if s>=0 else None}")
if s >= 0:
    log("  ctx:", cstr(s - 20, 120))

# REQUEST type enum near "Message is not a REQUEST"
s = img.find(b"Message is not a REQUEST")
log(f"not REQUEST @{hex(s)}")
# Look for nearby enum-ish strings
for key in [b"REQUEST", b"RESPONSE", b"INDICATION", b"CONFIRM", b"MessageType", b"kRequest", b"kIndication"]:
    pos = 0x6DF000
    hits = []
    while len(hits) < 5 and pos < 0x6E1000:
        i = img.find(key, pos, 0x6E1000)
        if i < 0:
            break
        hits.append((hex(i), cstr(i, 60)))
        pos = i + 1
    if hits:
        log(f"  {key!r}: {hits}")

# Missing fields checklist
log("\n=== MISSING for sendable 0x2f50 (do not invent) ===")
log("A) OEM app-layer header layout (field order/size): NOT recovered")
log("   candidates considered but unproven: reuse SIT 12B hdr; classic sipc_fmt_hdr 7B; custom magic")
log("B) Body 2 bytes content for flags=2: NOT recovered (zeros unproven)")
log("C) Token/seq/transaction assignment rules: NOT recovered")
log("D) Which oem_ipcN channel (0..7) stock uses for SIM_INIT: NOT recovered")
log("E) Whether link_header already set on oem_ipc iod (DT attrs): assumed yes (SIT oem uses fmt path) but channel# unknown")
log("\nSENDABLE? NO")
log("DONE")
