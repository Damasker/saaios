#!/usr/bin/env python3
"""MAIN RX consumer RE for catalog msgid 0x2f50 / 0x2f52.

Goal: recover inbound OEM app-frame layout from CP decode/dispatch
(not host encoder). Evidence only — no invented wire bytes.
"""
from __future__ import annotations

import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s, flush=True)
    lines.append(s)


def resolve(*parts: str) -> Path:
    wsl = Path("/mnt/c").joinpath(*parts)
    if wsl.exists():
        return wsl
    return Path("C:/").joinpath(*parts)


MAIN_CANDS = [
    resolve("Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    resolve("Users/Admin/Projects/saaios-som/fw-saaios-probe-b-modem-PATCHED-ready.bin"),
]
main_path = next((p for p in MAIN_CANDS if p.exists()), None)
if main_path is None:
    raise SystemExit(f"MAIN missing: {MAIN_CANDS}")
img = main_path.read_bytes()
VA0, MAIN = 0x40010000, 0x16C10


def va_of(o: int) -> int:
    return VA0 + (o - MAIN)


def off_of(va: int) -> int:
    return (va - VA0) + MAIN


def u16(o: int) -> int:
    return struct.unpack_from("<H", img, o)[0]


def u32(o: int) -> int:
    return struct.unpack_from("<I", img, o)[0]


def cstr(o: int, n: int = 96) -> str | None:
    if o < 0 or o >= len(img):
        return None
    s = bytearray()
    for i in range(o, min(len(img), o + n)):
        c = img[i]
        if c == 0:
            break
        if 32 <= c < 127:
            s.append(c)
        else:
            break
    return s.decode() if s else None


def ascii_near(o: int, radius: int = 48) -> list[str]:
    lo, hi = max(0, o - radius), min(len(img), o + radius)
    out, i = [], lo
    while i < hi:
        if 32 <= img[i] < 127:
            j = i
            while j < hi and 32 <= img[j] < 127:
                j += 1
            if j - i >= 6:
                out.append(img[i:j].decode())
            i = j
        else:
            i += 1
    return out


def find_all(needle: bytes, limit: int = 80) -> list[int]:
    hits, pos = [], 0
    while len(hits) < limit:
        i = img.find(needle, pos)
        if i < 0:
            break
        hits.append(i)
        pos = i + 1
    return hits


def thumb_movw_hits(imm: int, limit: int = 60) -> list[tuple[int, int]]:
    """Return (off, rd) for bare MOVW #imm (no same-rd MOVT filter here)."""
    hits = []
    i = 0
    while i < len(img) - 4 and len(hits) < limit:
        w, w2 = u16(i), u16(i + 2)
        if (w & 0xFBF0) == 0xF240 and (w & 0xF800) >= 0xE800:
            i_bit = (w >> 10) & 1
            imm4 = w & 0xF
            imm3 = (w2 >> 12) & 7
            rd = (w2 >> 8) & 0xF
            imm8 = w2 & 0xFF
            val = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
            if val == imm:
                # skip if immediate MOVT same rd within +16
                has_movt = False
                for j in range(i + 4, min(i + 20, len(img) - 4), 2):
                    ww, ww2 = u16(j), u16(j + 2)
                    if (ww & 0xFBF0) == 0xF2C0 and ((ww2 >> 8) & 0xF) == rd:
                        has_movt = True
                        break
                    if (ww & 0xF800) < 0xE800:
                        j -= 2
                        break
                if not has_movt:
                    hits.append((i, rd))
            i += 4
        elif (w & 0xF800) >= 0xE800:
            i += 4
        else:
            i += 2
    return hits


def dump_window(o: int, n: int = 0x80, label: str = "") -> None:
    log(f"--- {label} @{hex(o)} va={hex(va_of(o))} ---")
    end = min(len(img) - 4, o + n)
    i = o & ~1
    while i < end:
        w, w2 = u16(i), u16(i + 2)
        note = ""
        if (w & 0xFBF0) == 0xF240:
            i_bit = (w >> 10) & 1
            imm4 = w & 0xF
            imm3 = (w2 >> 12) & 7
            rd = (w2 >> 8) & 0xF
            imm8 = w2 & 0xFF
            imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
            note = f";MOVW r{rd},#{hex(imm)}"
            if imm in (0x2F50, 0x2F52, 0x2F57, 0x2FA1, 0xABCD, 0xF8, 12, 10, 2):
                note += " **"
            log(f" {hex(i)}: {w:04x} {w2:04x} {note}")
            i += 4
            continue
        if (w & 0xFBF0) == 0xF2C0:
            i_bit = (w >> 10) & 1
            imm4 = w & 0xF
            imm3 = (w2 >> 12) & 7
            rd = (w2 >> 8) & 0xF
            imm8 = w2 & 0xFF
            imm = (i_bit << 11) | (imm4 << 12) | (imm3 << 8) | imm8
            note = f";MOVT r{rd},#{hex(imm)}"
            log(f" {hex(i)}: {w:04x} {w2:04x} {note}")
            i += 4
            continue
        # LDRB Rt,[Rn,#imm] T1 0111 1 imm5 Rn Rt
        if (w & 0xF800) == 0x7800:
            imm5 = (w >> 6) & 0x1F
            rn = (w >> 3) & 7
            rt = w & 7
            note = f";LDRB r{rt},[r{rn},#{imm5}]"
            if imm5 in (0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 12):
                note += " hdr?"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        # LDRH Rt,[Rn,#imm] T1 1000 1 imm5 Rn Rt
        if (w & 0xF800) == 0x8800:
            imm5 = (w >> 6) & 0x1F
            rn = (w >> 3) & 7
            rt = w & 7
            note = f";LDRH r{rt},[r{rn},#{imm5*2}]"
            if imm5 * 2 in (0, 2, 4, 6, 8, 10):
                note += " hdr?"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        # CMP Rn,#imm8
        if (w & 0xF800) == 0x2800:
            rn = (w >> 8) & 7
            imm8 = w & 0xFF
            note = f";CMP r{rn},#{imm8}"
            log(f" {hex(i)}: {w:04x}      {note}")
            i += 2
            continue
        if (w & 0xF800) >= 0xE800:
            log(f" {hex(i)}: {w:04x} {w2:04x}")
            i += 4
        else:
            log(f" {hex(i)}: {w:04x}")
            i += 2


log(f"MAIN={main_path} size={len(img)}")
log(f"VA0={hex(VA0)} MAIN_OFF={hex(MAIN)}")

# --- A) Catalog + USIM table (known anchors) ---
CAT = 0x6DE740
log("\n=== A) Catalog SIM_INIT / VERIFYPIN ===")
for off, want in ((0x6DE740, 0x2F50), (0x6DE874, 0x2F52), (0x6DE724, 0x2F57)):
    body, mid, name_va, meta, rsp = u16(off), u16(off + 2), u32(off + 4), u32(off + 8), u16(off + 12)
    name = cstr(off_of(name_va)) if VA0 <= name_va < VA0 + 0x8000000 else None
    log(
        f"  @{hex(off)} body={body} msgid={hex(mid)} meta={hex(meta)} rsp={hex(rsp)} "
        f"name={name} raw28={' '.join(f'{b:02x}' for b in img[off:off+28])}"
    )
    assert mid == want

USIM_INIT = 0x10FB078
log("\n=== A2) USIM msgtable around SIM_INIT (16B stride) ===")
for o in range(USIM_INIT - 0x40, USIM_INIT + 0x60, 16):
    a, b, c, d = u32(o), u32(o + 4), u32(o + 8), u32(o + 12)
    sa = cstr(off_of(a)) if VA0 <= a < VA0 + 0x8000000 else None
    sb = cstr(off_of(b)) if VA0 <= b < VA0 + 0x8000000 else None
    log(f"  @{hex(o)} {hex(a)}|{hex(b)}|{hex(c)}|{hex(d)}  {sa or ''} {sb or ''}")

# --- B) RX path strings → litpool → code ---
log("\n=== B) OEM/SIT RX string sites ===")
RX_STRS = [
    b"[OEM][SIT] Received packet from channel %u, size %u",
    b"[OEM][IPC] Received ",
    b"Received packet from channel",
    b"Message is not a REQUEST",
    b"Unable to decode",
    b"Invalid message",
    b"GetHeader()->message_id",
    b"GetDataSpan().size()",
    b"oem_ipc_message_dispatcher.c",
    b"oem_ipc_message_utils.c",
    b"USIM <== SIM_INIT_REQ",
    b"USIM <== SIM_VERIFYPIN_REQ",
    b"Waiting for SIM_INIT_REQ",
]
for s in RX_STRS:
    hits = find_all(s, 8)
    log(f"  {s!r}: {[hex(h) for h in hits]}")
    for h in hits[:2]:
        va = va_of(h)
        # litpool refs to this VA
        refs = find_all(struct.pack("<I", va & 0xFFFFFFFF), 20)
        codeish = [r for r in refs if abs(r - h) > 0x20]
        log(f"    va={hex(va)} litrefs={len(refs)} nearby_codeish={[hex(r) for r in codeish[:8]]}")

# --- C) Bare MOVW #0x2f50 / #0x2f52 with LDRH header peeks nearby ---
log("\n=== C) Bare MOVW #0x2f50/#0x2f52 + nearby LDRH/LDRB (RX demux candidates) ===")
for imm, label in ((0x2F50, "SIM_INIT"), (0x2F52, "VERIFYPIN")):
    hits = thumb_movw_hits(imm, 40)
    log(f"  {label} bare MOVW #{hex(imm)}: n={len(hits)}")
    for o, rd in hits[:20]:
        # scan +/- 0x60 for LDRH/LDRB with small imm (header field reads)
        lo, hi = max(0, o - 0x60), min(len(img) - 2, o + 0x80)
        hdr_ops = []
        j = lo & ~1
        while j < hi:
            w = u16(j)
            if (w & 0xF800) == 0x8800:  # LDRH
                imm5 = (w >> 6) & 0x1F
                rn = (w >> 3) & 7
                rt = w & 7
                offb = imm5 * 2
                if offb <= 16:
                    hdr_ops.append(f"LDRH r{rt},[r{rn},#{offb}]@{hex(j)}")
            elif (w & 0xF800) == 0x7800:  # LDRB
                imm5 = (w >> 6) & 0x1F
                rn = (w >> 3) & 7
                rt = w & 7
                if imm5 <= 16:
                    hdr_ops.append(f"LDRB r{rt},[r{rn},#{imm5}]@{hex(j)}")
            if (w & 0xF800) >= 0xE800:
                j += 4
            else:
                j += 2
        ascii = ascii_near(o, 64)
        log(
            f"    @{hex(o)} va={hex(va_of(o))} r{rd} hdr_ops={hdr_ops[:8]} "
            f"ascii={ascii[:3]}"
        )
        if hdr_ops:
            dump_window(max(0, o - 0x40), 0xA0, f"movw_{label}@{hex(o)}")

# --- D) Catalog meta=0x10104 decode hypotheses vs RX ---
log("\n=== D) Catalog meta=0x10104 field census (SIM bank) ===")
# walk catalog stride 28 from 0x6de4xx..0x6debxx
meta_hist: dict[int, int] = {}
body_by_id = {}
for o in range(0x6DE400, 0x6DEC00, 28):
    body, mid, meta = u16(o), u16(o + 2), u32(o + 8)
    if 0x2F00 <= mid <= 0x2FFF:
        meta_hist[meta] = meta_hist.get(meta, 0) + 1
        name_va = u32(o + 4)
        body_by_id[mid] = (body, meta, cstr(off_of(name_va)) if VA0 <= name_va < VA0 + 0x8000000 else None)
for m, c in sorted(meta_hist.items(), key=lambda x: -x[1]):
    log(f"  meta={hex(m)} count={c} bytes={list(struct.pack('<I', m))}")
for mid in (0x2F50, 0x2F52, 0x2F57, 0x2F51, 0x2F40, 0x2F42):
    if mid in body_by_id:
        body, meta, name = body_by_id[mid]
        log(f"  id={hex(mid)} body={body} meta={hex(meta)} name={name}")

# --- E) Compare soft SIT VerifyPin (0x0201) vs OEM 0x2f52 body sizes ---
log("\n=== E) Soft SIT vs OEM catalog size cross-check ===")
log("  soft SIT VerifyPin 0x0201: total_len=38 => header12 + body26 (evidenced host)")
log("  OEM catalog VERIFYPIN 0x2f52: body/flags=10 — NOT equal to SIT body26")
log("  OEM catalog SIM_INIT 0x2f50: body/flags=2")
log("  => OEM body sizes are catalog-internal; do NOT copy SIT VerifyPin body as OEM")

# --- F) Typed kMessageInfo / GetHeader asserts ---
log("\n=== F) Typed IPC asserts (header/size) ===")
for s in (
    b"GetHeader()->message_id == kMessageId",
    b"GetDataSpan().size() == kMessageInfo",
    b"message_id == kMessageId",
):
    h = img.find(s)
    log(f"  {s!r} @{hex(h) if h>=0 else None}")
    if h >= 0:
        log(f"    ctx={cstr(h - 20, 120)!r}")
        refs = find_all(struct.pack("<I", va_of(h) & 0xFFFFFFFF), 30)
        log(f"    litrefs={[hex(r) for r in refs[:12]]}")

# --- G) Channel / link strings near OEM SIT RX ---
log("\n=== G) Channel / link clues ===")
for s in (
    b"oem_ipc",
    b"umts_ipc",
    b"IPC_OEM",
    b"IPC_FMT",
    b"channel %u",
    b"PROTOCOL_SIT",
    b"link_header",
    b"EXYNOS",
    b"sync_byte",
    b"0xABCD",
):
    hits = find_all(s, 15)
    log(f"  {s!r}: n={len(hits)} first={[hex(h) for h in hits[:6]]}")
    for h in hits[:2]:
        log(f"    @{hex(h)} {cstr(max(0,h-8), 80)!r}")

# --- H) Dispatcher / decode: LDRH msgid from buf+N near catalog walk ---
log("\n=== H) Catalog base VA litpool (0x406d7b30 SIM_INIT entry) ===")
for target in (0x406D7B30, 0x406D7AF8, 0x406D7C64, 0x6DE740 + VA0 - MAIN):
    # catalog entry VAs
    pass
for target in (0x406D7B30, 0x406D7C64, 0x406D7B14):
    refs = find_all(struct.pack("<I", target), 40)
    log(f"  lit {hex(target)}: n={len(refs)} {[hex(r) for r in refs[:12]]}")

# --- I) VERIFYPIN RX handler table (cross-check) ---
log("\n=== I) USIM <== SIM_VERIFYPIN_REQ table ===")
vp = img.find(b"USIM <== SIM_VERIFYPIN_REQ")
log(f"  str@{hex(vp) if vp>=0 else None} va={hex(va_of(vp)) if vp>=0 else None}")
if vp >= 0:
    name_va = va_of(vp)
    # find name_va in USIM table region
    needle = struct.pack("<I", name_va)
    for r in find_all(needle, 20):
        # expect msgid before name
        mid = u32(r - 4) if r >= 4 else 0
        handler = u32(r + 4)
        size = u32(r + 8)
        log(
            f"  tablehit@{hex(r)} mid={hex(mid)} handler={hex(handler)} size={hex(size)} "
            f"prev={hex(u32(r-8)) if r>=8 else 0}"
        )

# --- J) Verdict scaffold ---
log("\n=== VERDICT (evidence-only) ===")
log("Evidenced:")
log("  - Catalog SIM_INIT_REQ: msgid=0x2f50 body_hint=2 meta=0x10104 rsp=0")
log("  - Catalog SIM_VERIFYPIN_REQ: msgid=0x2f52 body_hint=10 meta=0x10104 rsp=0x2fa1")
log("  - USIM <== SIM_INIT_REQ table entry handler=0x43909d49 size_word=0x10000 (internal, ≠ body 2)")
log("  - Soft SIT VerifyPin ≠ OEM 0x2f52 body size")
log("Missing if still unrecovered after dump above:")
log("  - OEM app-layer header field order/size on RX buffer")
log("  - Exact 2-byte body contents for flags=2")
log("  - Token/seq rules")
log("  - Which oem_ipcN / whether umts_ipc can carry catalog 0x2f50")
log("DONE")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"wrote {OUT}")
