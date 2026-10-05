#!/usr/bin/env python3
"""Deep-dive: carved-cbd-1827b000 SIM_INIT + carve libsitril/rild_exynos for 0x2f50 encode."""
from __future__ import annotations

import struct
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
VDIR = ROOT / "fw/cdma-hunt/factory-td1a-vendor"
V = VDIR / "vendor.img"
OUT = ROOT / "tmp-vendor-2f50-host-hunt2.out"


def log(fh, *a):
    s = " ".join(str(x) for x in a)
    print(s, flush=True)
    fh.write(s + "\n")


def expand(b, off, n=140):
    a = off
    while a > 0 and 32 <= b[a - 1] < 127:
        a -= 1
    c = off
    while c < len(b) and b[c] and c - a < n:
        c += 1
    return b[a:c].decode("ascii", "ignore")


def find_all(b, n, limit=40):
    out, pos = [], 0
    while len(out) < limit:
        i = b.find(n, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def movz_hits(blob, imm, limit=30):
    hits = []
    for i in range(0, len(blob) - 3, 4):
        w = struct.unpack_from("<I", blob, i)[0]
        if (w & 0xFF800000) != 0x52800000:
            continue
        imm16 = (w >> 5) & 0xFFFF
        hw = (w >> 21) & 3
        if (imm16 << (hw * 16)) == imm:
            hits.append(i)
            if len(hits) >= limit:
                break
    return hits


def elf_size_guess(data, elf_off):
    e_phoff = struct.unpack_from("<Q", data, elf_off + 32)[0]
    e_shoff = struct.unpack_from("<Q", data, elf_off + 40)[0]
    e_phentsize = struct.unpack_from("<H", data, elf_off + 54)[0]
    e_phnum = struct.unpack_from("<H", data, elf_off + 56)[0]
    e_shentsize = struct.unpack_from("<H", data, elf_off + 58)[0]
    e_shnum = struct.unpack_from("<H", data, elf_off + 60)[0]
    end = elf_off + 64
    if e_phoff and e_phnum and e_phentsize:
        end = max(end, elf_off + e_phoff + e_phnum * e_phentsize)
        for i in range(e_phnum):
            off = elf_off + e_phoff + i * e_phentsize
            if off + 56 > len(data):
                break
            p_offset = struct.unpack_from("<Q", data, off + 8)[0]
            p_filesz = struct.unpack_from("<Q", data, off + 32)[0]
            end = max(end, elf_off + p_offset + p_filesz)
    if e_shoff and e_shnum and e_shentsize and e_shnum < 10_000:
        end = max(end, elf_off + e_shoff + e_shnum * e_shentsize)
        for i in range(min(e_shnum, 512)):
            off = elf_off + e_shoff + i * e_shentsize
            if off + 64 > len(data):
                break
            sh_offset = struct.unpack_from("<Q", data, off + 24)[0]
            sh_size = struct.unpack_from("<Q", data, off + 32)[0]
            if 0 < sh_size < 0x20000000:
                end = max(end, elf_off + sh_offset + sh_size)
    return min(end, len(data)) - elf_off


def find_elf_before(b, pos, back=64 * 1024 * 1024):
    lo = max(0, pos - back)
    j = pos
    while j >= lo:
        k = b.rfind(b"\x7fELF", lo, j + 1)
        if k < 0:
            return None
        if k + 20 <= len(b) and b[k + 4] == 2 and b[k + 5] == 1:
            if struct.unpack_from("<H", b, k + 18)[0] == 0xB7:
                et = struct.unpack_from("<H", b, k + 16)[0]
                if et in (2, 3):
                    return k
        j = k - 1
    return None


def scan_blob(fh, label, blob):
    log(fh, f"\n=== {label} size={len(blob)} ===")
    for nb in (
        b"SIM_INIT_REQ",
        b"SIM_INIT",
        b"IpcTxSimInit",
        b"SimInitMessage",
        b"/dev/oem_ipc0",
        b"/dev/oem_ipc",
        b"/dev/umts_ipc0",
        b"BuildOemSimRequest",
        b"OemSim",
        b"0x2f50",
        b"IPC_OEM",
        b"oem_ipc_message",
    ):
        hits = find_all(blob, nb, 8)
        log(fh, f"  {nb!r}: {len(hits)}")
        for h in hits[:4]:
            log(fh, f"    @{hex(h)}: {expand(blob, h)[:110]!r}")
    m50 = movz_hits(blob, 0x2F50)
    m52 = movz_hits(blob, 0x2F52)
    m201 = movz_hits(blob, 0x0201)
    log(fh, f"  MOVZ #0x2f50: {[hex(x) for x in m50]}")
    log(fh, f"  MOVZ #0x2f52: {[hex(x) for x in m52]}")
    log(fh, f"  MOVZ #0x0201 count={len(m201)} sample={[hex(x) for x in m201[:5]]}")
    # dynsym-ish strings for encode
    for nb in (b"IpcTx", b"encode", b"Serialize", b"SitOem", b"IoChannel"):
        c = blob.count(nb)
        if c:
            log(fh, f"  count {nb!r}={c} first@{hex(blob.find(nb))}: {expand(blob, blob.find(nb))[:80]!r}")


def main():
    with OUT.open("w", encoding="utf-8") as fh:
        # 1) carved-cbd-1827b000 SIM_INIT context
        p = VDIR / "carved-cbd" / "carved-cbd-1827b000.elf"
        blob = p.read_bytes()
        log(fh, f"=== carved-cbd-1827b000.elf size={len(blob)} ===")
        for nb in (b"SIM_INIT", b"SIM_", b"/dev/oem_ipc", b"/dev/umts", b"cbd", b"ModemData"):
            for h in find_all(blob, nb, 12):
                log(fh, f"  {nb!r}@{hex(h)}: {expand(blob, h)[:120]!r}")
        scan_blob(fh, "carved-cbd-1827b000", blob)

        # 2) carve libsitril.so / rild_exynos / sit-stream from vendor via path strings
        data = V.read_bytes()
        log(fh, f"\nvendor size={len(data)}")

        targets = [
            (b"libsitril.so", "carved-libsitril"),
            # property line is not the .so itself; look for ELF with BuildOemSim + umts_ipc0
            (b"_ZN18ProtocolSimBuilder18BuildOemSimRequestEiPhi", "carved-sitril-builder"),
            (b"/dev/umts_ipc0  open error: errno %d", "carved-umts-open-err"),
            (b"rild_exynos", "carved-rild-hint"),
        ]
        carved = {}
        for needle, name in targets:
            for h in find_all(data, needle, 3):
                elf = find_elf_before(data, h)
                log(
                    fh,
                    f"needle {needle!r}@{hex(h)} elf={hex(elf) if elf else None} "
                    f"delta={h-elf if elf else None} ctx={expand(data, h)[:90]!r}",
                )
                if elf is None:
                    continue
                if elf in carved:
                    continue
                sz = elf_size_guess(data, elf)
                # clamp absurd
                if sz < 4096 or sz > 12 * 1024 * 1024:
                    log(fh, f"  skip elf@{hex(elf)} bad size={sz}")
                    continue
                outp = VDIR / f"{name}-{hex(elf)}.so"
                outp.write_bytes(data[elf : elf + sz])
                carved[elf] = outp
                log(fh, f"  WROTE {outp} size={sz}")

        for elf, outp in carved.items():
            scan_blob(fh, outp.name, outp.read_bytes())

        # 3) Also check already-known research libsitril if present
        mr = Path(
            "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
        )
        if mr.exists():
            scan_blob(fh, f"research {mr.name}", mr.read_bytes())

        log(fh, "\n=== HARD WALL CHECK ===")
        log(fh, "Need: SO that opens oem_ipc* AND encodes msgid 0x2f50 (MOVZ or catalog write)")
        log(fh, "SitOem oem_ipc0 = protobuf only (already closed)")
        log(fh, "BuildOemSimRequest = SIT umts_ipc dialect (already closed)")
        log(fh, "DONE")


if __name__ == "__main__":
    main()
