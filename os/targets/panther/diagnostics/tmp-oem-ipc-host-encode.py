#!/usr/bin/env python3
"""RE host OEM IPC outer frame for SIM_INIT 0x2f50 from cbd/libs/MAIN/CPIF.

No live I/O. No secrets. No invented wire bytes in verdict.
"""
from __future__ import annotations

import struct
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects")
MAIN = ROOT / "saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
# fallback
if not MAIN.exists():
    MAIN = ROOT / "saaios-som/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
LIBS = [
    ROOT / "saaios-modem-research/os/targets/panther/diagnostics/libsitril.so",
    ROOT / "saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so",
    ROOT / "saaios/os/build/e4-www/libsec-ril.so",
    ROOT / "saaios/os/build/libsec-ril.so",
    ROOT / "saaios/dist/panther/cbd-extract/raw-cbd",
]
VA0 = 0x40010000
MAIN_OFF = 0x16C10


def log(*a):
    print(*a, flush=True)


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def cstr(b, o, n=120):
    if o < 0 or o >= len(b):
        return None
    s = bytearray()
    for i in range(o, min(len(b), o + n)):
        c = b[i]
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def find_all(hay: bytes, needle: bytes, limit=40):
    out = []
    pos = 0
    while len(out) < limit:
        i = hay.find(needle, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def dump_ascii_win(b, i, before=48, after=96):
    start = max(0, i - before)
    end = min(len(b), i + after)
    frag = bytes(x if 32 <= x < 127 else 0x2E for x in b[start:end])
    return frag.decode("ascii", "replace")


def aarch64_movz_imm(w):
    if (w & 0xFF800000) == 0x52800000:
        return (w >> 5) & 0xFFFF, w & 0x1F
    return None


def thumb_movw_imm(lo, hi):
    # MOVW Rd, #imm16: 0xF240 | ...
    # encoding T3: i:imm4:imm3:Rd:imm8
    if (hi & 0xFBF0) == 0xF240:
        i = (hi >> 10) & 1
        imm4 = hi & 0xF
        imm3 = (lo >> 12) & 7
        rd = (lo >> 8) & 0xF
        imm8 = lo & 0xFF
        imm = (i << 11) | (imm4 << 12) | (imm3 << 8) | imm8
        return rd, imm
    return None


def scan_movz(b, imm_want, limit=30):
    hits = []
    for i in range(0, len(b) - 4, 4):
        w = struct.unpack_from("<I", b, i)[0]
        r = aarch64_movz_imm(w)
        if r and r[0] == imm_want:
            hits.append((i, r[1], w))
            if len(hits) >= limit:
                break
    return hits


# ---------- libs / cbd ----------
log("=== binary inventory ===")
for p in [MAIN] + LIBS:
    log(f"  {p}: exists={p.exists()} size={p.stat().st_size if p.exists() else '-'}")

for p in LIBS:
    if not p.exists():
        continue
    b = p.read_bytes()
    log(f"\n=== {p.name} key strings ===")
    for needle in [
        b"oem_ipc",
        b"/dev/oem",
        b"SIM_INIT",
        b"OemIpc",
        b"IPC message",
        b"Unable to encode",
        b"message_id",
        b"magicNumber",
        b"SIPC5",
        b"EXYNOS",
        b"oem_ipc_message",
        b"EncodeIpc",
        b"DecodeIpc",
        b"IpcHeader",
        b"hdr_len",
        b"transaction",
    ]:
        hits = find_all(b, needle, 8)
        if not hits:
            continue
        log(f"  {needle!r} x{len(hits)}")
        for i in hits[:3]:
            log(f"    @{hex(i)} {dump_ascii_win(b, i)}")

    for imm, label in [(0x2F50, "0x2f50"), (0x2F52, "0x2f52"), (0x2F57, "0x2f57"), (0xABCD, "EXYNOS_SYNC"), (0xF8, "SIPC5_START?")]:
        if imm == 0xF8:
            continue
        hits = scan_movz(b, imm, 20)
        log(f"  MOVZ #{label}: {len(hits)} eg {[(hex(i), rd) for i, rd, _ in hits[:6]]}")

# ---------- MAIN catalog + encode neighborhood ----------
if not MAIN.exists():
    log("MAIN missing; stop")
    raise SystemExit(1)

img = MAIN.read_bytes()
log(f"\n=== MAIN size {len(img)} ===")

# catalog SIM_INIT
cat = 0x6DE740
log(f"catalog @ {hex(cat)}: {' '.join(f'{x:02x}' for x in img[cat:cat+28])}")
flags = u16(img, cat)
msgid = u16(img, cat + 2)
meta = u32(img, cat + 8)
log(f"  flags/body={flags} msgid={hex(msgid)} meta={hex(meta)}")

# Find refs to encode error strings and dump nearby thumb for msgid stores
log("\n=== MAIN encode/decode string xref neighborhoods ===")
keys = [
    b"[OEM][IPC] Unable to encode IPC message",
    b"[OEM][IPC] Unable to get encoded size",
    b"[OEM][IPC] Message ID not found",
    b"[OEM][IPC] Message is not a REQUEST",
    b"[OEM][SIT] Sending data length %u to channel %u",
    b"[OEM][SIT] Received packet from channel %u, size %u",
    b"oem_ipc_message_utils.c",
    b"oem_ipc_message_dispatcher.c",
]
for k in keys:
    offs = find_all(img, k, 5)
    log(f"\nstr {k!r}: {[hex(o) for o in offs]}")
    for o in offs[:2]:
        # scan ±0x200 for ADR/LDR of this string is hard; dump nearby code-looking MOVW #0x2fxx
        win_lo = max(0, o - 0x400)
        win_hi = min(len(img) - 2, o + 0x80)
        movs = []
        for i in range(win_lo & ~1, win_hi, 2):
            lo = u16(img, i)
            hi = u16(img, i + 2) if i + 3 < len(img) else 0
            # try both endian pairs for thumb
            r = thumb_movw_imm(lo, hi)
            if r and (0x2F00 <= r[1] <= 0x2FFF or r[1] in (0x7F, 0xABCD, 0xAA)):
                movs.append((hex(i), hex(r[1]), f"r{r[0]}"))
            r2 = thumb_movw_imm(hi, lo)
            if r2 and (0x2F00 <= r2[1] <= 0x2FFF):
                movs.append((hex(i), hex(r2[1]), f"r{r2[0]}-swap?"))
        log(f"  near-str MOVW OEM-ish: {movs[:20]}")

# Hunt for potential outer header layouts near catalog consumer:
# look for sequences that store u16 msgid then u16 length patterns in OEM band
log("\n=== Hunt packed header prototypes (len,msgid) near catalog ===")
# Search for literal 0x2f50 as halfword in .rodata near headers/examples
hits_hw = []
pos = 0
while len(hits_hw) < 40:
    i = img.find(b"\x50\x2f", pos)
    if i < 0:
        break
    # require aligned or near other 0x2fxx
    ctx = img[max(0, i - 16) : i + 24]
    if b"\x52\x2f" in ctx or b"\x57\x2f" in ctx or b"\x58\x2f" in ctx or i == cat + 2:
        hits_hw.append(i)
    pos = i + 1
log(f"0x2f50 halfword near other SIM OEM ids: {len(hits_hw)}")
for i in hits_hw[:15]:
    log(f"  @{hex(i)} {' '.join(f'{x:02x}' for x in img[i-8:i+20])} ascii={cstr(img, i-8, 40)!r}")

# Compare meta field meanings across catalog
log("\n=== Catalog meta histogram (SIM bank) ===")
from collections import Counter

meta_c = Counter()
for off in range(0x6DE60C, 0x6DEA00, 28):
    meta_c[u32(img, off + 8)] += 1
    fl = u16(img, off)
    mid = u16(img, off + 2)
    name = cstr(img, u32(img, off + 4) - VA0 - (0 if False else 0))
    # name ptr is VA; convert
    name_va = u32(img, off + 4)
    name_off = name_va - VA0 + MAIN_OFF if False else name_va - 0x41000000  # wrong
# Fix VA mapping: strings in MAIN often at file offset == (va - VA0 + MAIN_OFF)? 
# Prior scripts used name as absolute file? Looking at raw: +4 = f3 05 03 41 => VA 0x410305f3
# file offset for that string: previous out said name='SIM_INIT_REQ' so they mapped somehow.

def va_to_off(va):
    # try common maps
    cands = [va - VA0 + MAIN_OFF, va - 0x40000000, va - 0x41000000 + 0x100000, va & 0xFFFFFF]
    for c in cands:
        s = cstr(img, c, 40)
        if s and s.startswith("SIM_"):
            return c, s
    # brute: search string
    return None, None


# re-parse with name search
log("SIM entries flags/meta:")
for off in range(0x6DE724, 0x6DE890, 28):
    fl = u16(img, off)
    mid = u16(img, off + 2)
    name_va = u32(img, off + 4)
    meta = u32(img, off + 8)
    # find name in image
    # Prior: name_va 0x410305f3 -> need offset
    # From framing.out: name at that VA works via their VA map
    off_name = name_va - VA0  # if VA0 maps file start of MAIN region?
    # Actually MAIN file starts with bootloader; MAIN_OFF=0x16C10 is MAIN start corresponding to VA0
    off_name = (name_va - VA0) + MAIN_OFF
    name = cstr(img, off_name, 48)
    rsp = u16(img, off + 0xC) if fl else 0
    log(f"  @{hex(off)} fl={fl:#x} id={mid:#x} meta={meta:#x} +0c={u32(img,off+0xc):#x} +10={u32(img,off+0x10):#x} +14={u32(img,off+0x14):#x} +18={u32(img,off+0x18):#x} name={name}")

# Host SIT FMT header (known from sit-sim-status) vs OEM catalog flags
log("\n=== Compare known SIT host header vs OEM catalog ===")
log("SIT umts_ipc0: type0 + pad0 + id_le16 + len_le16 + token_le32 (+body)")
log("OEM catalog flags field = encoded body size hint (INIT=2 VERIFYPIN=0xa)")
log("If OEM reused SIT outer: total_len = 12 + flags => INIT=14 VERIFYPIN=22")
log("soft SIT VerifyPin total was 38 — NOT matching OEM flags — confirms separate framing")

# Search MAIN for struct sizes / "encoded size" computation constants near utils
log("\n=== 'encoded size' / encode IPC xref by string pool adjacency ===")
for k in [b"encoded size", b"Unable to encode IPC message", b"Unable to get encoded size"]:
    for o in find_all(img, k, 3):
        # dump 64 bytes before as potential LDR pools
        pool = img[o - 64 : o]
        log(f"  pool before {k!r} @{hex(o)}: {' '.join(f'{x:02x}' for x in pool)}")

# Look for host-side documentation in saaios trees (read-only scan of .c)
log("\n=== DONE inputs for verdict ===")
log("kernel: PROTOCOL_SIT oem_ipc write gets EXYNOS 12B prepended by CPIF; userspace payload = OEM app frame")
log("cbd: opens umts_boot0/ramdump only — NOT oem_ipc encode path")
log("need: OEM app header fields (type/seq/len/msgid/token?) evidenced from encode path or stock capture")
