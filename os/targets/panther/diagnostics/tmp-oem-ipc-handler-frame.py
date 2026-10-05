#!/usr/bin/env python3
"""Deep RE: OEM IPC app-layer frame from CP encode/decode + SIM_INIT handler.

No live I/O. Document only evidenced fields.
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


def off_of(va):
    return (va - VA0) + MAIN


def cstr(o, n=80):
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


def find_litpool_refs(target_va, band=None, limit=40):
    """Find Thumb literal-pool words equal to target_va."""
    hits = []
    lo, hi = (0, len(img) - 4) if band is None else band
    t = struct.pack("<I", target_va & 0xFFFFFFFF)
    pos = lo
    while len(hits) < limit:
        i = img.find(t, pos, hi)
        if i < 0:
            break
        if i % 4 == 0 or True:
            hits.append(i)
        pos = i + 1
    return hits


def disasm_thumb_movw_movt(start, length=0x180):
    """Lightweight dump of MOVW/MOVT/LDR immediates."""
    out = []
    end = min(len(img) - 4, start + length)
    i = start & ~1
    while i < end:
        w = u16(i)
        w2 = u16(i + 2)
        # 32-bit Thumb if Fxxx and next
        if (w & 0xF800) >= 0xE800:
            # MOVW: 11110 i 10 0 1 0 0 imm4 | 0 imm3 Rd imm8
            if (w & 0xFBF0) == 0xF240:
                i_bit = (w >> 10) & 1
                imm4 = w & 0xF
                imm3 = (w2 >> 12) & 7
                rd = (w2 >> 8) & 0xF
                imm8 = w2 & 0xFF
                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                out.append((i, f"MOVW r{rd},#{hex(imm)}"))
            elif (w & 0xFBF0) == 0xF2C0:  # MOVT
                i_bit = (w >> 10) & 1
                imm4 = w & 0xF
                imm3 = (w2 >> 12) & 7
                rd = (w2 >> 8) & 0xF
                imm8 = w2 & 0xFF
                imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
                out.append((i, f"MOVT r{rd},#{hex(imm)}"))
            i += 4
        else:
            # LDR Rt, [pc, #imm] T1: 01001 Rt imm8
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                val = u32(lit) if lit + 4 <= len(img) else 0
                out.append((i, f"LDR r{rt},[pc+{imm}] lit@{hex(lit)}={hex(val)} '{cstr(off_of(val) if VA0<=val<=VA0+len(img) else -1,40)}'"))
            i += 2
    return out


# 1) String VAs and litpool refs
strings = {
    "encode_fail": b"[OEM][IPC] Unable to encode IPC message",
    "enc_size_fail": b"[OEM][IPC] Unable to get encoded size",
    "msgid_nf": b"[OEM][IPC] Message ID not found",
    "not_req": b"[OEM][IPC] Message is not a REQUEST",
    "sit_send": b"[OEM][SIT] Sending data length %u to channel %u",
    "sit_recv": b"[OEM][SIT] Received packet from channel %u, size %u",
    "host_not_ready": b"[OEM][SIT] Host interface is not ready",
    "utils": b"oem_ipc_message_utils.c",
    "disp": b"oem_ipc_message_dispatcher.c",
    "invalid_msg": b"[OEM][IPC] Invalid message",
    "alloc": b"[OEM][IPC] Unable to allocate size %u",
}

log("=== String VA + litpool refs ===")
for name, s in strings.items():
    o = img.find(s)
    if o < 0:
        log(f"{name}: MISSING")
        continue
    va = va_of(o)
    refs = find_litpool_refs(va, limit=20)
    log(f"{name}: off={hex(o)} va={hex(va)} litrefs={len(refs)} {[hex(r) for r in refs[:8]]}")
    # For each litref, search back ~0x200 for code that might LDR it
    for r in refs[:4]:
        # scan back for LDR pc-relative that could hit this pool
        code_hits = []
        for i in range(max(0, r - 0x200), r, 2):
            w = u16(i)
            if (w & 0xF800) == 0x4800:
                rt = (w >> 8) & 7
                imm = (w & 0xFF) << 2
                lit = ((i + 4) & ~2) + imm
                if lit == r or abs(lit - r) <= 2:
                    code_hits.append((hex(i), f"r{rt}"))
        log(f"  lit@{hex(r)} LDR-pc hits: {code_hits[:6]}")
        if code_hits:
            fn = int(code_hits[0][0], 16)
            for off, txt in disasm_thumb_movw_movt(max(0, fn - 0x40), 0x120):
                if "MOVW" in txt or "MOVT" in txt or "LDR" in txt:
                    log(f"    {hex(off)}: {txt}")

# 2) SIM_INIT handler (prior: off=0x3910958)
h = 0x3910958
log(f"\n=== SIM_INIT handler @ {hex(h)} va={hex(va_of(h))} ===")
log("bytes:", dump(h, 96))
for off, txt in disasm_thumb_movw_movt(h, 0x200):
    log(f"  {hex(off)}: {txt}")

# Look for LDRB/LDRH from message pointer patterns: common OEM hdr offsets
# Search handler for immediates 0,1,2,4,6,8,10,12 as struct field access — via disasm of STR/LDR
log("\n=== handler field-access immediates (LDR*/STR* imm) first 0x200 ===")
i = h & ~1
end = h + 0x200
while i < end:
    w = u16(i)
    w2 = u16(i + 2)
    if (w & 0xF800) >= 0xE800:
        # LDRB.W Rt,[Rn,#imm12] F89
        if (w & 0xFFF0) == 0xF890:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x40:
                log(f"  {hex(i)}: LDRB.W r{rt},[r{rn},#{imm}]")
        # LDRH.W
        if (w & 0xFFF0) == 0xF8B0:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x40:
                log(f"  {hex(i)}: LDRH.W r{rt},[r{rn},#{imm}]")
        # LDR.W
        if (w & 0xFFF0) == 0xF8D0:
            rt = (w2 >> 12) & 0xF
            rn = w & 0xF
            imm = w2 & 0xFFF
            if imm <= 0x40:
                log(f"  {hex(i)}: LDR.W r{rt},[r{rn},#{imm}]")
        i += 4
    else:
        # LDRB Rt,[Rn,#imm5]
        if (w & 0xF800) == 0x7800:
            imm = (w >> 6) & 0x1F
            rt = w & 7
            rn = (w >> 3) & 7
            log(f"  {hex(i)}: LDRB r{rt},[r{rn},#{imm}]")
        # LDRH
        if (w & 0xF800) == 0x8800:
            imm = ((w >> 6) & 0x1F) << 1
            rt = w & 7
            rn = (w >> 3) & 7
            log(f"  {hex(i)}: LDRH r{rt},[r{rn},#{imm}]")
        i += 2

# 3) Find decode path that checks "not a REQUEST" — likely reads type field
not_req = img.find(b"[OEM][IPC] Message is not a REQUEST")
not_req_va = va_of(not_req)
refs = find_litpool_refs(not_req_va)
log(f"\n=== not-REQUEST decode (type field) va={hex(not_req_va)} refs={refs[:10]} ===")
for r in refs[:6]:
    # find function containing LDR to this
    for i in range(max(0, r - 0x400), r, 2):
        w = u16(i)
        if (w & 0xF800) == 0x4800:
            imm = (w & 0xFF) << 2
            lit = ((i + 4) & ~2) + imm
            if lit == r:
                log(f"code@{hex(i)} uses lit@{hex(r)}")
                # dump MOVW and LDRB nearby for type compare
                for off, txt in disasm_thumb_movw_movt(i - 0x80, 0x180):
                    log(f"  {hex(off)}: {txt}")
                # field accesses
                j = i - 0x80
                while j < i + 0x100:
                    w = u16(j)
                    w2 = u16(j + 2)
                    if (w & 0xF800) >= 0xE800:
                        if (w & 0xFFF0) in (0xF890, 0xF8B0, 0xF8D0):
                            rt = (w2 >> 12) & 0xF
                            rn = w & 0xF
                            imm = w2 & 0xFFF
                            op = {0xF890: "LDRB", 0xF8B0: "LDRH", 0xF8D0: "LDR"}[w & 0xFFF0]
                            if imm <= 0x30:
                                log(f"  {hex(j)}: {op}.W r{rt},[r{rn},#{imm}]")
                        j += 4
                    else:
                        j += 2

# 4) Compare other OEM msgs that host might send - look for sit-stream SIT builders packing
# also search MAIN for magic 0x7F style Shannon IPC (classic FMT starts with 0x7F)
log("\n=== classic Shannon IPC 0x7F near OEM strings? ===")
band = img[0x6D0000:0x6E2000]
c7f = band.count(b"\x7f")
log(f"0x7f count in OEM band: {c7f}")

# 5) Host libsitril: dump table around 0x95660 from prior
lib = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so").read_bytes()
log("\n=== libsitril table @0x95660 ===")
o = 0x95660
log(dump(o - 0x20, 0x80))
# interpret as records
for i in range(o - 0x40, o + 0x80, 0x18):
    if i < 0:
        continue
    mid = u16.__wrapped__(lib, i) if False else struct.unpack_from("<H", lib, i)[0]
    # try stride 24 from prior hex: 50 2f 23 00 ...
    
log("records-ish:")
pos = 0x95648
for _ in range(6):
    chunk = lib[pos : pos + 24]
    mid = struct.unpack_from("<H", chunk, 0)[0]
    log(f"  @{hex(pos)} mid={hex(mid)} raw={' '.join(f'{x:02x}' for x in chunk)}")
    pos += 24

# 6) Search libsitril for /dev/oem or open oem
for n in [b"/dev/oem_ipc", b"oem_ipc0", b"OEM_IPC", b"IpcChannel", b"channel %u"]:
    idx = lib.find(n)
    log(f"lib {n!r}: {hex(idx) if idx>=0 else None}")

ss = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so").read_bytes()
for n in [b"/dev/oem_ipc", b"oem_ipc0", b"OEM", b"2f50", b"SIM_INIT"]:
    idx = ss.find(n)
    log(f"ss {n!r}: {hex(idx) if idx>=0 else None} count={ss.count(n)}")

log("\nDONE")
