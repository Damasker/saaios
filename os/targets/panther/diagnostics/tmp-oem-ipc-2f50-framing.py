#!/usr/bin/env python3
"""RE OEM IPC framing for SIM_INIT_REQ (0x2f50) vs soft SIT VerifyPin (0x0201).

Goal: evidence for wire layout / channel — do NOT invent bytes.
No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

VA = 0x40010000
MAIN = 0x16C10
PATH = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
STREAM = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
RIL = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
img = PATH.read_bytes()


def log(*a):
    print(*a, flush=True)


def u16(o):
    return struct.unpack_from("<H", img, o)[0]


def u32(o):
    return struct.unpack_from("<I", img, o)[0]


def va_of(o):
    return VA + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA)


def cstr(off, n=100):
    if off < 0 or off >= len(img):
        return None
    s = bytearray()
    for i in range(off, min(len(img), off + n)):
        c = img[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


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


def find_movw_movt_va(target_va, band=(0x600000, 0x2000000)):
    lo, hi = target_va & 0xFFFF, (target_va >> 16) & 0xFFFF
    refs = []
    o = band[0]
    while o < band[1] - 8:
        r = movw(o)
        if r and r[0] == lo:
            p = o + 4
            end = min(o + 0x30, band[1] - 4)
            while p < end:
                t = movt(p)
                if t and t[1] == r[1] and t[0] == hi:
                    refs.append(o)
                    break
                hw = u16(p)
                p += 4 if (hw & 0xF800) in (0xE800, 0xF000, 0xF800) else 2
        o += 2
    return refs


def find_movw_imm(imm, band, lim=40):
    out = []
    o = band[0]
    while o < band[1] - 4 and len(out) < lim:
        r = movw(o)
        if r and r[0] == imm:
            out.append((o, r[1]))
        o += 2
    return out


def dump_bytes(o, n=0x40):
    b = img[o : o + n]
    return " ".join(f"{x:02x}" for x in b)


# --- 1) Full catalog record layout around SIM_INIT ---
log("=== Catalog raw around SIM_INIT @0x6de740 ===")
for base in range(0x6DE700, 0x6DE8A0, 0x1C):
    # try 0x1C stride (prior walk used name@+4 from flags)
    pass

# Prior parse: @entry = flags:u16 msgid:u16 name_va:u32 meta:u32 ...
# Prove stride by walking consecutive SIM_ entries
log("\n=== Consecutive SIM_* catalog records (auto stride) ===")
entries = []
o = 0x6DE600
while o < 0x6DEA00:
    w = u32(o)
    so = w - VA + MAIN
    s = cstr(so, 60) if 0 <= so < len(img) else None
    if s and s.startswith("SIM_"):
        flags = u16(o - 4)
        msgid = u16(o - 2)
        meta = u32(o + 4)
        # dump next 0x18 bytes after name ptr for hidden fields
        raw = dump_bytes(o - 4, 0x20)
        entries.append((o - 4, flags, msgid, s, meta, raw))
    o += 4

for e in entries:
    off, flags, msgid, s, meta, raw = e
    log(f"  @{hex(off)} flags={hex(flags)} id={hex(msgid)} meta={hex(meta)} {s}")
    log(f"    raw: {raw}")

# Diff deltas between entry starts
starts = [e[0] for e in entries]
if len(starts) > 1:
    deltas = sorted(set(starts[i + 1] - starts[i] for i in range(len(starts) - 1)))
    log(f"\nentry start deltas: {deltas}")

# Focus INIT / INFO / VERIFYPIN
log("\n=== Focus INIT/INFO/VERIFYPIN fields ===")
for want in (0x2F50, 0x2F57, 0x2F52):
    for e in entries:
        if e[2] == want:
            off = e[0]
            log(f"\nid={hex(want)} @{hex(off)}")
            log(f"  +0x00..0x2f: {dump_bytes(off, 0x30)}")
            # interpret words
            for i in range(0, 0x30, 4):
                w = u32(off + i)
                so = w - VA + MAIN
                s = cstr(so, 50) if 0 <= so < len(img) else None
                log(f"  +{i:02x} u32={hex(w)} str={s!r}")


# --- 2) Who references catalog entry VAs / msgid constants near OEM strings ---
log("\n=== Refs to SIM_INIT catalog VA / name VA ===")
init_off = 0x6DE740
init_va = va_of(init_off)
# name ptr at +4 from entry start (flags at 0, msgid at 2, name at 4)
name_va = u32(init_off + 4)
name_off = off_va(name_va)
log(f"entry VA={hex(init_va)} name_va={hex(name_va)} name={cstr(name_off)!r}")
for label, tva in (
    ("entry", init_va),
    ("name", name_va),
    ("meta_field", va_of(init_off + 8)),
):
    refs = find_movw_movt_va(tva)
    log(f"  {label} {hex(tva)} movw/movt refs={len(refs)} {[hex(x) for x in refs[:12]]}")

# --- 3) OEM IPC encode/decode strings → nearby code ---
log("\n=== OEM IPC encode/decode string sites + nearby MOVW msgids ===")
needles = [
    b"[OEM][IPC] Unable to encode IPC message",
    b"[OEM][IPC] Unable to get encoded size",
    b"[OEM][IPC] Message ID not found",
    b"[OEM][IPC] Received ",
    b"[OEM][IPC] Message is not a REQUEST",
    b"[OEM][SIT] Received packet from channel",
    b"[OEM][SIT] Sending data length",
    b"[OEM][SIT] Host interface is not ready",
]
for nd in needles:
    pos = 0
    while True:
        i = img.find(nd, pos)
        if i < 0:
            break
        log(f"\n  str@{hex(i)}: {cstr(i, 70)!r}")
        # find LDR of this string VA in nearby code is hard; scan ±0x200 for MOVW #0x2f50/52/57
        band = (max(0, i - 0x800), min(len(img), i + 0x800))
        # actually string is in rodata; find refs to string VA
        sva = va_of(i)
        refs = find_movw_movt_va(sva, (0x600000, 0xC00000))
        if not refs:
            refs = find_movw_movt_va(sva, (0x1000000, 0x2000000))
        log(f"    refs to str: {[hex(x) for x in refs[:8]]}")
        for r in refs[:3]:
            # dump code window
            log(f"    code@{hex(r)}: {dump_bytes(r, 0x40)}")
            mw = find_movw_imm(0x2F50, (r - 0x80, r + 0x120), lim=8)
            mw2 = find_movw_imm(0x2F52, (r - 0x80, r + 0x120), lim=8)
            if mw or mw2:
                log(f"      nearby 2f50={[hex(x[0]) for x in mw]} 2f52={[hex(x[0]) for x in mw2]}")
        pos = i + 1


# --- 4) Decode meta=0x10104 pattern across catalog ---
log("\n=== meta field histogram (SIM catalog window) ===")
meta_hist = {}
for e in entries:
    meta_hist[e[4]] = meta_hist.get(e[4], 0) + 1
for m, c in sorted(meta_hist.items()):
    log(f"  meta={hex(m)} count={c}")

log("\n=== flags as possible body size (INIT/INFO/PIN neighbors) ===")
for e in entries:
    if e[2] in (0x2F50, 0x2F57, 0x2F52, 0x2F53, 0x2F54, 0x2F55, 0x2F56, 0x2F42, 0x2F40):
        log(f"  {e[3]} id={hex(e[2])} flags={e[1]} ({hex(e[1])})")


# --- 5) sit-stream / libsitril: any 0x2f50 / SIM_INIT / OEM ---
def scan_so(path: Path, label: str):
    if not path.exists():
        log(f"\n=== {label}: MISSING {path} ===")
        return
    data = path.read_bytes()
    log(f"\n=== {label} ({path.name} {len(data)} bytes) ===")
    for key in (
        b"SIM_INIT",
        b"SimInit",
        b"0x2f50",
        b"OEM",
        b"oem_ipc",
        b"BuildSim",
        b"VerifyPin",
        b"sitInformSimInit",
    ):
        n = data.count(key)
        if n:
            # show a few contexts
            pos = 0
            shown = 0
            while shown < 5:
                i = data.find(key, pos)
                if i < 0:
                    break
                ctx = data[i : i + 50]
                ctx = "".join(chr(c) if 32 <= c < 127 else "." for c in ctx)
                log(f"  {key!r} x{n} eg@{hex(i)} {ctx!r}")
                shown += 1
                pos = i + 1
                if shown >= 1 and key in (b"OEM", b"BuildSim"):
                    break
    # raw halfword 0x2f50 in so (unlikely)
    needle = struct.pack("<H", 0x2F50)
    hits = []
    start = 0
    while len(hits) < 10:
        i = data.find(needle, start)
        if i < 0:
            break
        hits.append(i)
        start = i + 1
    log(f"  u16 0x2f50 raw hits (first10)={[hex(h) for h in hits]}")


scan_so(STREAM, "sit-stream")
scan_so(RIL, "libsitril")


# --- 6) USIM inbound handler for SIM_INIT: expected payload size ---
log("\n=== USIM <== SIM_INIT_REQ string + nearby table/handler hints ===")
i = img.find(b"USIM <== SIM_INIT_REQ")
log(f"  str@{hex(i)} va={hex(va_of(i))}")
# Prior: handler table @0x10fb07c area
for off in (0x10FB070, 0x10FB078, 0x10FB07C, 0x10FB080, 0x10FB088):
    log(f"  table@{hex(off)}: {dump_bytes(off, 0x20)}")
    for i in range(0, 0x20, 4):
        w = u32(off + i)
        so = w - VA + MAIN
        s = cstr(so, 40) if 0 <= so < len(img) else None
        log(f"    +{i:02x} {hex(w)} {s!r}")


# --- 7) Compare soft SIT VerifyPin builder length vs OEM VERIFYPIN flags ---
log("\n=== Soft SIT VerifyPin (known) vs OEM catalog VERIFYPIN ===")
log("  soft: SIT id 0x0201 len=38 on umts_ipc0 (type0 hdr)")
log("  OEM catalog SIM_VERIFYPIN_REQ id=0x2f52 flags=0xa meta=0x10104")
log("  => parallel ID spaces; soft VerifyPin does NOT prove OEM wire format")
log("  soft transport reuse = umts_ipc0 SIT framing ONLY if 0x2f50 is a SIT id (not evidenced)")

log("\nDONE")
