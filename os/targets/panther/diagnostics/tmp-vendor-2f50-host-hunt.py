#!/usr/bin/env python3
"""Hunt factory-td1a vendor.img for catalog OEM 0x2f50 / SIM_INIT encoder (offline)."""
from __future__ import annotations

import struct
from pathlib import Path

OUT = Path(__file__).with_suffix(".out")
V = Path(__file__).resolve().parent / "fw/cdma-hunt/factory-td1a-vendor/vendor.img"
CARVE = V.parent


def log(*a):
    print(*a, flush=True)


def find_all(b: bytes, n: bytes, limit: int = 30):
    out, pos = [], 0
    while len(out) < limit:
        i = b.find(n, pos)
        if i < 0:
            break
        out.append(i)
        pos = i + 1
    return out


def expand(b: bytes, off: int, n: int = 120) -> str:
    a = off
    while a > 0 and 32 <= b[a - 1] < 127:
        a -= 1
    c = off
    while c < len(b) and b[c] and c - a < n:
        c += 1
    return b[a:c].decode("ascii", "ignore")


def find_elf_before(b: bytes, pos: int, back: int = 32 * 1024 * 1024):
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


def movz_hits(blob: bytes, imm: int, limit: int = 20):
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


def main():
    with OUT.open("w", encoding="utf-8") as fh:
        def out(*a):
            s = " ".join(str(x) for x in a)
            log(s)
            fh.write(s + "\n")

        b = V.read_bytes()
        out(f"vendor size={len(b)}")

        needles = [
            b"SIM_INIT_REQ",
            b"SIM_INIT",
            b"IpcTxSimInit",
            b"SimInitMessage",
            b"0x2f50",
            b"/dev/oem_ipc0",
            b"/dev/oem_ipc",
            b"oem_ipc_message",
            b"IPC_OEM",
            b"OemSimRequest",
            b"BuildOemSim",
            b"SitOemHandler",
            b"libsitril",
            b"rild_exynos",
            b"libsec-ril",
            b"umts_ipc0",
        ]
        for n in needles:
            hits = find_all(b, n, 12)
            out(f"\n{n!r}: sample={len(hits)}")
            for h in hits[:6]:
                out(f"  @{hex(h)}: {expand(b, h)[:100]!r}")

        out("\n=== ELF near interesting strings ===")
        for n in (
            b"SIM_INIT_REQ",
            b"IpcTxSimInit",
            b"SimInitMessage",
            b"/dev/oem_ipc0",
            b"libsitril.so",
            b"rild_exynos",
            b"umts_ipc0",
        ):
            for h in find_all(b, n, 5):
                elf = find_elf_before(b, h)
                delta = (h - elf) if elf is not None else None
                out(
                    f"  needle={n!r} @{hex(h)} elf={hex(elf) if elf is not None else None} delta={delta}"
                )

        out("\n=== carved oemipc/cbd MOVZ / strings ===")
        paths = list(CARVE.glob("carved-oemipc-*.so")) + list(
            (CARVE / "carved-cbd").glob("*.elf")
        )
        for p in sorted(paths):
            blob = p.read_bytes()
            m50 = movz_hits(blob, 0x2F50)
            m52 = movz_hits(blob, 0x2F52)
            out(
                f"  {p.name}: size={len(blob)} movz2f50={ [hex(x) for x in m50] }"
                f" movz2f52={[hex(x) for x in m52]}"
                f" SIM_INIT={blob.find(b'SIM_INIT')} oem_ipc={blob.find(b'/dev/oem_ipc')}"
            )
            for nb in (
                b"SitOem",
                b"Ping",
                b"SerializeToArray",
                b"SIM_",
                b"ModemData",
                b"initialMessageHeader",
                b"protobuf",
                b"IpcTx",
            ):
                if blob.find(nb) >= 0:
                    out(f"    has {nb!r}")

        # Broader: any ELF containing both oem_ipc and MOVZ 0x2f50?
        out("\n=== scan all /dev/oem_ipc* ELF islands for MOVZ 0x2f50 ===")
        oem_hits = find_all(b, b"/dev/oem_ipc", 40)
        seen = set()
        for h in oem_hits:
            elf = find_elf_before(b, h, back=8 * 1024 * 1024)
            if elf is None or elf in seen:
                continue
            seen.add(elf)
            # size guess crude: next ELF or +4MB
            end = min(len(b), elf + 4 * 1024 * 1024)
            nxt = b.find(b"\x7fELF", elf + 4, end)
            if nxt > elf:
                end = nxt
            blob = b[elf:end]
            m50 = movz_hits(blob, 0x2F50, 5)
            has_sim = blob.find(b"SIM_INIT") >= 0
            out(
                f"  elf@{hex(elf)} size~={len(blob)} oem_at={hex(h)} "
                f"movz2f50={[hex(x) for x in m50]} SIM_INIT={has_sim} "
                f"str={expand(b, h)[:60]!r}"
            )

        out("\n=== verdict scaffolding ===")
        out("Encoder SO/symbol for catalog 0x2f50: report only if MOVZ+oem_ipc+encode path clear")
        out("DONE")


if __name__ == "__main__":
    main()
