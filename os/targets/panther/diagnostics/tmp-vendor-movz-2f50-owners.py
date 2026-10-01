#!/usr/bin/env python3
"""Map vendor MOVZ #0x2f50 sites to nearest ELF + interesting strings."""
from __future__ import annotations

import struct
from pathlib import Path

VENDOR = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/"
    "cdma-hunt/factory-td1a-vendor/vendor.img"
)
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "tmp-vendor-movz-2f50-owners.out"
)


def movz_imm16(data: bytes, imm: int) -> list[int]:
    hits = []
    base = 0x52800000 | ((imm & 0xFFFF) << 5)
    for rd in range(32):
        w = struct.pack("<I", base | rd)
        start = 0
        while True:
            j = data.find(w, start)
            if j < 0:
                break
            if j % 4 == 0:
                hits.append(j)
            start = j + 4
    return sorted(set(hits))


def elf_base(data: bytes, hit: int, back: int = 0x400000) -> int | None:
    start = max(0, hit - back)
    i = hit
    while i >= start:
        i = data.rfind(b"\x7fELF", start, i + 1)
        if i < 0:
            return None
        if data[i + 4] == 2 and struct.unpack_from("<H", data, i + 16)[0] in (2, 3):
            return i
    return None


def interesting(data: bytes, off: int, win: int = 0x1000) -> list[str]:
    chunk = data[max(0, off - win) : off + win]
    out = []
    cur = bytearray()
    s = 0
    for i, b in enumerate(chunk):
        if 32 <= b < 127:
            if not cur:
                s = i
            cur.append(b)
        else:
            if len(cur) >= 8:
                t = cur.decode()
                low = t.lower()
                if any(
                    k in low
                    for k in (
                        "sim",
                        "oem",
                        "ipc",
                        "ril",
                        "cbd",
                        "init",
                        "sit",
                        "modem",
                        "catalog",
                    )
                ):
                    out.append(t[:90])
            cur = bytearray()
    # unique preserve order
    seen = set()
    uniq = []
    for t in out:
        if t not in seen:
            seen.add(t)
            uniq.append(t)
    return uniq[:10]


def main() -> None:
    data = VENDOR.read_bytes()
    mz = movz_imm16(data, 0x2F50)
    lines = [f"MOVZ#0x2f50 count={len(mz)}"]
    by_elf: dict[int | None, list[int]] = {}
    for off in mz:
        b = elf_base(data, off)
        by_elf.setdefault(b, []).append(off)
    for b, offs in sorted(by_elf.items(), key=lambda x: (x[0] is None, x[0] or 0)):
        lines.append(f"\nelf@{b and hex(b)} sites={len(offs)} first={[hex(x) for x in offs[:6]]}")
        # pick middle site for strings
        mid = offs[len(offs) // 2]
        lines.append(f"  sample_near@{mid:#x}: {interesting(data, mid)}")
        # also check oem_ipc presence in island if elf known
        if b is not None:
            nxt = data.find(b"\x7fELF", b + 0x1000)
            end = min(len(data), nxt if nxt > 0 else b + 0x300000)
            blob = data[b:end]
            lines.append(
                f"  island~={len(blob)} oem_ipc={b'/dev/oem_ipc' in blob} "
                f"umts_ipc={b'/dev/umts_ipc' in blob} SIM_INIT_REQ={b'SIM_INIT_REQ' in blob} "
                f"BuildOemSim={b'BuildOemSimRequest' in blob}"
            )
    # research libsitril
    for p in [
        Path(
            "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
        ),
        Path(
            "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/libsitril.so"
        ),
    ]:
        lines.append(f"\nlibsitril {p}: exists={p.exists()}")
        if p.exists():
            d = p.read_bytes()
            m = movz_imm16(d, 0x2F50)
            lines.append(
                f"  size={len(d)} MOVZ#0x2f50={len(m)} sample={[hex(x) for x in m[:8]]} "
                f"oem={b'/dev/oem_ipc' in d} umts={b'/dev/umts_ipc' in d} "
                f"SIM_INIT_REQ={b'SIM_INIT_REQ' in d}"
            )
    text = "\n".join(lines) + "\n"
    OUT.write_text(text, encoding="utf-8")
    print(text)


if __name__ == "__main__":
    main()
