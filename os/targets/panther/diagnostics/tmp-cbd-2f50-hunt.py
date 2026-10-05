#!/usr/bin/env python3
"""Offline hunt: cbd / libsitril ELFs in vendor.img for catalog 0x2f50 emitter."""
from __future__ import annotations

import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
lines: list[str] = []


def log(s: str = "") -> None:
    print(s, flush=True)
    lines.append(s)


def p(*parts: str) -> Path:
    wsl = Path("/mnt/c").joinpath(*parts)
    return wsl if wsl.exists() else Path("C:\\").joinpath(*parts)


VENDOR = p(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
)


def find_all(hay: bytes, needle: bytes) -> list[int]:
    out, start = [], 0
    while True:
        i = hay.find(needle, start)
        if i < 0:
            break
        out.append(i)
        start = i + 1
    return out


def u16(b: bytes, o: int) -> int:
    return struct.unpack_from("<H", b, o)[0]


def u32(b: bytes, o: int) -> int:
    return struct.unpack_from("<I", b, o)[0]


# Locate interesting path strings, carve nearby ELF, scan for 0x2f50
targets = [
    b"/vendor/bin/cbd",
    b"/vendor/bin/hw/rild",
    b"/vendor/bin/rild",
    b"libsitril.so",
    b"/vendor/lib64/libsitril.so",
    b"/vendor/lib64/libsec-ril.so",
    b"sitInformSimInit",
    b"SIM_INIT_REQ",
    b"SIM_INIT or SIM_RESET",
]

log("=== locate path strings in vendor.img ===")
hits: dict[bytes, list[int]] = {}
chunk = 8 * 1024 * 1024
overlap = 128
prev = b""
with VENDOR.open("rb") as f:
    pos = 0
    while True:
        data = f.read(chunk)
        if not data:
            break
        buf = prev + data
        base = pos - len(prev)
        for nd in targets:
            start = 0
            while True:
                i = buf.find(nd, start)
                if i < 0:
                    break
                abs_off = base + i
                hits.setdefault(nd, []).append(abs_off)
                if len(hits[nd]) <= 5:
                    ctx = bytes(c if 32 <= c < 127 else 0x2E for c in buf[max(0, i - 24) : i + 64])
                    log(f"  {nd.decode()} @{hex(abs_off)} {ctx.decode()}")
                start = i + 1
        prev = data[-overlap:]
        pos += len(data)

for nd in targets:
    log(f"count {nd.decode()}={len(hits.get(nd, []))}")


def carve_elf_at(abs_off: int, max_back: int = 4 * 1024 * 1024, max_size: int = 12 * 1024 * 1024) -> tuple[int, bytes] | None:
    """Read vendor slice ending at abs_off, find last ELF magic, return (elf_off, bytes)."""
    start = max(0, abs_off - max_back)
    with VENDOR.open("rb") as f:
        f.seek(start)
        buf = f.read(abs_off - start + 64)
    rel = buf.rfind(b"\x7fELF")
    if rel < 0:
        return None
    elf_off = start + rel
    # read ELF header to estimate size from section headers if possible
    with VENDOR.open("rb") as f:
        f.seek(elf_off)
        hdr = f.read(64)
        if len(hdr) < 64 or hdr[:4] != b"\x7fELF":
            return None
        e_shoff = struct.unpack_from("<Q", hdr, 40)[0]
        e_shentsize = struct.unpack_from("<H", hdr, 58)[0]
        e_shnum = struct.unpack_from("<H", hdr, 60)[0]
        # rough size: shoff + shnum*entsize, or fall back
        guess = e_shoff + e_shnum * e_shentsize + 0x1000
        if guess < 0x1000 or guess > max_size:
            guess = min(max_size, 2 * 1024 * 1024)
        f.seek(elf_off)
        blob = f.read(guess)
        return elf_off, blob


def scan_blob(label: str, blob: bytes, elf_off: int) -> None:
    log(f"\n--- {label} elf@{hex(elf_off)} size={len(blob)} ---")
    for nd in (
        b"SIM_INIT_REQ",
        b"SIM_INIT",
        b"sitInformSimInit",
        b"/dev/oem_ipc",
        b"/dev/umts_ipc",
        b"0x2f50",
        b"2f50",
    ):
        offs = find_all(blob, nd)
        log(f"  {nd.decode()}: {len(offs)}")
        for o in offs[:4]:
            ctx = bytes(c if 32 <= c < 127 else 0x2E for c in blob[max(0, o - 12) : o + 48])
            log(f"    @{hex(o)} {ctx.decode()}")
    # MOVZ W,#0x2f50
    movz = []
    for o in range(0, len(blob) - 3, 4):
        w = u32(blob, o)
        if (w & 0xFFC00000) in (0x52800000, 0xD2800000):
            imm = (w >> 5) & 0xFFFF
            if imm == 0x2F50:
                movz.append(o)
    log(f"  MOVZ #0x2f50: {len(movz)} {[hex(x) for x in movz[:12]]}")
    # LE u16 0x2f50 count (noisy)
    u16c = sum(1 for o in range(0, len(blob) - 1, 2) if u16(blob, o) == 0x2F50)
    log(f"  u16 LE 0x2f50 count: {u16c}")


# Carve from best path hits
carve_plan = []
if hits.get(b"/vendor/bin/cbd"):
    carve_plan.append(("cbd", hits[b"/vendor/bin/cbd"][0]))
if hits.get(b"/vendor/lib64/libsitril.so"):
    carve_plan.append(("libsitril", hits[b"/vendor/lib64/libsitril.so"][0]))
elif hits.get(b"libsitril.so"):
    # prefer longer path-like; use first
    carve_plan.append(("libsitril.so-str", hits[b"libsitril.so"][0]))
if hits.get(b"/vendor/bin/hw/rild"):
    carve_plan.append(("rild-hw", hits[b"/vendor/bin/hw/rild"][0]))
if hits.get(b"/vendor/bin/rild"):
    carve_plan.append(("rild", hits[b"/vendor/bin/rild"][0]))

# Also search for bare filename cbd as ELF DT_NEEDED / path
# Try known TD1A layout: often /vendor/bin/cbd is in same partition

for label, off in carve_plan:
    got = carve_elf_at(off)
    if not got:
        log(f"FAIL carve {label} @{hex(off)}")
        continue
    elf_off, blob = got
    scan_blob(label, blob, elf_off)

# Extra: scan whole vendor for MOVZ #0x2f50 near "SIM" ascii within 256B — expensive; sample by
# scanning only around libsitril / cbd carve regions already done.

# Check prior carved SitOem again for completeness message
log("\n=== cross-check: MAIN USIM log still waits for AP SIM_INIT_REQ ===")
MAIN = p(
    "Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"
)
main = MAIN.read_bytes()
for nd in (b"Waiting for SIM_INIT_REQ", b"USIM_NOT_INITIALISED", b"USIM <== SIM_INIT_REQ"):
    offs = find_all(main, nd)
    log(f"  {nd.decode()}: {len(offs)} first={[hex(o) for o in offs[:3]]}")

log("\n=== FINAL ===")
log("SitOem protobuf ↛ catalog 0x2f50 (proven separate dialects).")
log("No soft SIM_INIT send (frame still incomplete; no invent).")
log("Bearer: no try.")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
log(f"Wrote {OUT}")
