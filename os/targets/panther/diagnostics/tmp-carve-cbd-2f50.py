#!/usr/bin/env python3
"""Offline carve factory cbd ELF from vendor.img; RE SIM_INIT / oem_ipc write.

Do NOT start cbd. Scan carved ELF for 0x2f50 / SIM_INIT_REQ / oem_ipc encode.
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
    return Path("C:\\").joinpath(*parts)


VENDOR = resolve(
    "Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
)
CARVE_DIR = VENDOR.parent / "carved-cbd"
CARVE_DIR.mkdir(parents=True, exist_ok=True)

if not VENDOR.exists():
    raise SystemExit(f"vendor.img missing: {VENDOR}")

log(f"VENDOR={VENDOR} size={VENDOR.stat().st_size}")


def find_all_file(path: Path, needle: bytes, limit: int = 50) -> list[int]:
    hits = []
    chunk = 8 * 1024 * 1024
    overlap = max(len(needle), 64)
    prev = b""
    with path.open("rb") as f:
        pos = 0
        while len(hits) < limit:
            data = f.read(chunk)
            if not data:
                break
            buf = prev + data
            base = pos - len(prev)
            start = 0
            while len(hits) < limit:
                i = buf.find(needle, start)
                if i < 0:
                    break
                hits.append(base + i)
                start = i + 1
            prev = buf[-overlap:]
            pos += len(data)
    return hits


def u16(b: bytes, o: int) -> int:
    return struct.unpack_from("<H", b, o)[0]


def u32(b: bytes, o: int) -> int:
    return struct.unpack_from("<I", b, o)[0]


def carve_elf_near(path: Path, hint_off: int, back: int = 0x200000, fwd: int = 0x800000) -> tuple[int, bytes] | None:
    """Find ELF magic before hint; return (elf_off, bytes up to next ELF or size cap)."""
    start = max(0, hint_off - back)
    end = min(path.stat().st_size, hint_off + fwd)
    with path.open("rb") as f:
        f.seek(start)
        blob = f.read(end - start)
    # search ELF magics before hint within blob
    rel_hint = hint_off - start
    best = None
    pos = 0
    while True:
        i = blob.find(b"\x7fELF", pos, rel_hint + 1)
        if i < 0:
            break
        best = i
        pos = i + 1
    if best is None:
        return None
    elf_off = start + best
    # size: e_shoff+e_shentsize*e_shnum if sane, else until next ELF or 8MB
    with path.open("rb") as f:
        f.seek(elf_off)
        hdr = f.read(0x40)
    if len(hdr) < 0x40 or hdr[:4] != b"\x7fELF":
        return None
    ei_class = hdr[4]
    if ei_class == 2:  # ELF64
        e_shoff = struct.unpack_from("<Q", hdr, 0x28)[0]
        e_shentsize = struct.unpack_from("<H", hdr, 0x3A)[0]
        e_shnum = struct.unpack_from("<H", hdr, 0x3C)[0]
        e_phoff = struct.unpack_from("<Q", hdr, 0x20)[0]
        e_phentsize = struct.unpack_from("<H", hdr, 0x36)[0]
        e_phnum = struct.unpack_from("<H", hdr, 0x38)[0]
    else:
        e_shoff = struct.unpack_from("<I", hdr, 0x20)[0]
        e_shentsize = struct.unpack_from("<H", hdr, 0x2E)[0]
        e_shnum = struct.unpack_from("<H", hdr, 0x30)[0]
        e_phoff = struct.unpack_from("<I", hdr, 0x1C)[0]
        e_phentsize = struct.unpack_from("<H", hdr, 0x2A)[0]
        e_phnum = struct.unpack_from("<H", hdr, 0x2C)[0]
    size_guess = 0
    if 0 < e_shnum < 200 and e_shentsize in (40, 64) and e_shoff > 0:
        size_guess = e_shoff + e_shentsize * e_shnum
    if 0 < e_phnum < 50 and e_phentsize in (32, 56):
        # also use max p_offset+p_filesz
        with path.open("rb") as f:
            f.seek(elf_off + e_phoff)
            ph = f.read(e_phentsize * e_phnum)
        for n in range(e_phnum):
            ent = ph[n * e_phentsize : (n + 1) * e_phentsize]
            if ei_class == 2:
                p_offset = struct.unpack_from("<Q", ent, 0x08)[0]
                p_filesz = struct.unpack_from("<Q", ent, 0x20)[0]
            else:
                p_offset = struct.unpack_from("<I", ent, 0x04)[0]
                p_filesz = struct.unpack_from("<I", ent, 0x10)[0]
            size_guess = max(size_guess, p_offset + p_filesz)
    if size_guess < 0x1000 or size_guess > 16 * 1024 * 1024:
        size_guess = min(4 * 1024 * 1024, path.stat().st_size - elf_off)
    with path.open("rb") as f:
        f.seek(elf_off)
        data = f.read(size_guess)
    return elf_off, data


def scan_elf(label: str, data: bytes, out_path: Path | None = None) -> None:
    log(f"\n=== scan {label} size={len(data)} ===")
    if out_path:
        out_path.write_bytes(data)
        log(f"  wrote {out_path}")
    # ELF basics
    if data[:4] != b"\x7fELF":
        log("  NOT ELF")
        return
    log(f"  class={data[4]} machine={u16(data, 18)} type={u16(data, 16)}")
    needles = [
        b"SIM_INIT_REQ",
        b"SIM_INIT",
        b"SIM_VERIFYPIN",
        b"/dev/oem_ipc",
        b"oem_ipc0",
        b"oem_ipc",
        b"IpcTxSimInit",
        b"SimInitMessage",
        b"sitInformSimInit",
        b"0x2f50",
        b"2f50",
        b"umts_boot",
        b"umts_ramdump",
        b"umts_ipc",
        b"IPC_OEM",
        b"encode",
    ]
    for nd in needles:
        hits = []
        start = 0
        while len(hits) < 5:
            i = data.find(nd, start)
            if i < 0:
                break
            hits.append(i)
            start = i + 1
        if hits:
            ctx = data[hits[0] : hits[0] + 60]
            ctx_s = "".join(chr(c) if 32 <= c < 127 else "." for c in ctx)
            log(f"  {nd!r} count>={len(hits)} first@{hex(hits[0])}: {ctx_s}")
        else:
            log(f"  {nd!r}: 0")
    # MOVZ Wd,#0x2f50 : encoding 0x5285EA0W (aarch64) — MOVZ Wd, #imm16, LSL#0
    # imm16=0x2f50 -> (0x2f50<<5) | Rd | 0x52800000
    movz = []
    for rd in range(32):
        enc = 0x52800000 | (0x2F50 << 5) | rd
        pat = struct.pack("<I", enc)
        start = 0
        while len(movz) < 20:
            i = data.find(pat, start)
            if i < 0:
                break
            movz.append((i, rd))
            start = i + 4
    log(f"  AArch64 MOVZ #0x2f50: {len(movz)} {[hex(a) for a,_ in movz[:12]]}")
    # also 0x2f52
    movz2 = []
    for rd in range(32):
        enc = 0x52800000 | (0x2F52 << 5) | rd
        pat = struct.pack("<I", enc)
        start = 0
        while len(movz2) < 10:
            i = data.find(pat, start)
            if i < 0:
                break
            movz2.append(i)
            start = i + 4
    log(f"  AArch64 MOVZ #0x2f52: {len(movz2)} {[hex(a) for a in movz2[:8]]}")
    u16c = data.count(b"\x50\x2f")
    log(f"  u16 LE 0x2f50 raw count={u16c} (noisy)")


# Path string hunt
log("=== path-string hits in vendor.img ===")
for nd in [
    b"/vendor/bin/cbd",
    b"vendor/bin/cbd",
    b"/vendor/bin/hw/rild",
    b"/vendor/bin/rild",
    b"cbd_main",
    b"CBD version",
]:
    hits = find_all_file(VENDOR, nd, limit=10)
    log(f"  {nd!r}: {len(hits)} {[hex(h) for h in hits[:6]]}")

cbd_path_hits = find_all_file(VENDOR, b"/vendor/bin/cbd", limit=20)
# Also search for ELF that contains distinctive cbd strings
log("\n=== distinctive cbd string hunt ===")
cbd_markers = [
    b"cbd: boot stage",
    b"[cbd]",
    b"umts_boot0",
    b"/dev/umts_boot0",
    b"CP boot",
    b"cbd_protocol",
    b"exynos_boot",
]
marker_hits: dict[bytes, list[int]] = {}
for nd in cbd_markers:
    marker_hits[nd] = find_all_file(VENDOR, nd, limit=8)
    log(f"  {nd!r}: {[hex(h) for h in marker_hits[nd][:5]]}")

# Carve from best hints: path string + umts_boot0 (cbd opens boot)
hints = []
for h in cbd_path_hits:
    hints.append(("path_cbd", h))
for h in marker_hits.get(b"/dev/umts_boot0", []):
    hints.append(("umts_boot0", h))
for h in marker_hits.get(b"[cbd]", []):
    hints.append(("tag_cbd", h))
for h in marker_hits.get(b"cbd: boot stage", []):
    hints.append(("boot_stage", h))

seen_elf = set()
carved = []
for label, hint in hints[:12]:
    log(f"\n--- carve near {label} @{hex(hint)} ---")
    res = carve_elf_near(VENDOR, hint)
    if not res:
        log("  no ELF found before hint")
        continue
    elf_off, data = res
    if elf_off in seen_elf:
        log(f"  ELF @{hex(elf_off)} already carved")
        continue
    seen_elf.add(elf_off)
    outp = CARVE_DIR / f"carved-cbd-{elf_off:x}.elf"
    scan_elf(f"{label}@{hex(hint)} -> ELF@{hex(elf_off)}", data, outp)
    carved.append((elf_off, outp, data))

# If nothing carved, try finding standalone file named cbd in ext4 by string "cbd" + ELF nearby differently
if not carved:
    log("\n=== fallback: search ELF headers containing /dev/umts_boot0 in first 2MB after ELF ===")
    # scan for ELF then check content — expensive; sample every ELF near boot0
    boot_hits = marker_hits.get(b"/dev/umts_boot0", [])
    for h in boot_hits[:5]:
        res = carve_elf_near(VENDOR, h, back=0x400000, fwd=0x100000)
        if res:
            elf_off, data = res
            if elf_off not in seen_elf:
                seen_elf.add(elf_off)
                outp = CARVE_DIR / f"carved-bootish-{elf_off:x}.elf"
                scan_elf(f"boot0@{hex(h)} -> ELF@{hex(elf_off)}", data, outp)
                carved.append((elf_off, outp, data))

log(f"\n=== carved summary: {len(carved)} ELF(s) ===")
any_2f50 = False
any_oem = False
any_sim_init_req = False
for elf_off, outp, data in carved:
    has_2f50 = (0x52800000 | (0x2F50 << 5))  # check any rd
    movz_any = False
    for rd in range(32):
        if struct.pack("<I", 0x52800000 | (0x2F50 << 5) | rd) in data:
            movz_any = True
            break
    has_oem = b"/dev/oem_ipc" in data or b"oem_ipc0" in data
    has_sir = b"SIM_INIT_REQ" in data
    log(f"  @{hex(elf_off)} {outp.name}: MOVZ#0x2f50={movz_any} oem_ipc={has_oem} SIM_INIT_REQ={has_sir}")
    any_2f50 |= movz_any
    any_oem |= has_oem
    any_sim_init_req |= has_sir

log("\n=== VERDICT ===")
log(f"cbd ELF carved (offline): {len(carved)}")
log(f"SIM_INIT_REQ string in carved: {any_sim_init_req}")
log(f"MOVZ #0x2f50 in carved: {any_2f50}")
log(f"oem_ipc open string in carved: {any_oem}")
log("Encoder for catalog 0x2f50 from cbd: " + ("maybe — inspect above" if any_2f50 or any_sim_init_req else "NOT found"))
log("Did NOT start cbd/rild.")

OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"Wrote {OUT}", flush=True)
