#!/usr/bin/env python3
"""Broader factory-td1a vendor hunt: ANY binary island for catalog OEM dialect.

Looks past cbd/sitril/SitOem for SIM_INIT_REQ / 0x2f50 / IpcTxSimInit /
oem_ipc catalog encode. No invent. Offline only.
"""
from __future__ import annotations

import struct
from pathlib import Path

VENDOR = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/"
    "cdma-hunt/factory-td1a-vendor/vendor.img"
)
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "tmp-vendor-broad-2f50-hunt.out"
)

NEEDLES = [
    b"SIM_INIT_REQ",
    b"IpcTxSimInit",
    b"SimInitMessage",
    b"sitInformSimInit",
    b"WAIT_FOR_INIT",
    b"USIM_WAIT_FOR_INIT",
    b"SIM_START_STACK",
    b"oem_ipc_message",
    b"IPC_OEM",
    b"OemIpcRecord",
    b"0x2f50",
    b"/dev/oem_ipc0",
    b"/dev/oem_ipc1",
    b"/dev/oem_ipc",
    b"SIM_INIT",
]


def find_elf_base(data: bytes, hit: int, back: int = 0x800000) -> int | None:
    """Walk back from hit for ELF magic; prefer closest plausible base."""
    start = max(0, hit - back)
    best = None
    i = hit
    while i >= start:
        i = data.rfind(b"\x7fELF", start, i + 1)
        if i < 0:
            break
        if i + 0x40 > len(data):
            continue
        # ELF64 LE ET_DYN/ET_EXEC
        if data[i + 4] != 2:  # EI_CLASS 64
            continue
        et = struct.unpack_from("<H", data, i + 16)[0]
        if et not in (2, 3):  # EXEC/DYN
            continue
        best = i
        # keep walking for closer? prefer closest = first from hit
        break
    return best


def movz_imm16(data: bytes, imm: int) -> list[int]:
    """ARM64 MOVZ Wd, #imm16 (sf=0, hw=0): 0x5280_0000 | (imm<<5) | Rd — scan all Rd."""
    hits = []
    # opcode bits [31:21]=0x294 (MOVZ 32-bit), imm16 in [20:5]
    # word = 0x52800000 | (imm16 << 5) | rd
    base = 0x52800000 | ((imm & 0xFFFF) << 5)
    for rd in range(32):
        w = struct.pack("<I", base | rd)
        start = 0
        while True:
            j = data.find(w, start)
            if j < 0:
                break
            if (j % 4) == 0:
                hits.append(j)
            start = j + 4
    return hits


def ctx(data: bytes, off: int, n: int = 96) -> str:
    a = max(0, off - 24)
    b = min(len(data), off + n)
    return bytes(c if 32 <= c < 127 else 46 for c in data[a:b]).decode("ascii", "replace")


def main() -> None:
    lines: list[str] = []
    size = VENDOR.stat().st_size
    lines.append(f"vendor size={size}")
    # full mmap via read — 665MB ok
    data = VENDOR.read_bytes()
    lines.append(f"loaded {len(data)}")

    needle_hits: dict[bytes, list[int]] = {}
    for n in NEEDLES:
        hits = []
        start = 0
        while len(hits) < 80:
            i = data.find(n, start)
            if i < 0:
                break
            hits.append(i)
            start = i + 1
        needle_hits[n] = hits
        lines.append(f"\n=== {n!r} count={len(hits)} ===")
        for h in hits[:12]:
            lines.append(f"  @{h:#x}: {ctx(data, h)}")

    # For each /dev/oem_ipc* hit, find ELF island and scan MOVZ #0x2f50 / SIM_INIT_REQ
    oem_hits = sorted(set(needle_hits[b"/dev/oem_ipc0"] + needle_hits[b"/dev/oem_ipc1"] + needle_hits[b"/dev/oem_ipc"]))
    lines.append(f"\n=== OEM_IPC ELF ISLANDS ({len(oem_hits)} string hits) ===")
    seen_bases: set[int] = set()
    for h in oem_hits:
        base = find_elf_base(data, h)
        if base is None:
            lines.append(f"  hit@{h:#x} NO_ELF_BASE str={ctx(data,h,60)}")
            continue
        if base in seen_bases:
            continue
        seen_bases.add(base)
        # size heuristic: next ELF or +8MB
        nxt = data.find(b"\x7fELF", base + 0x1000)
        end = min(len(data), base + 0x800000 if nxt < 0 else nxt)
        # clamp: many vendor ELFs are <4MB
        blob = data[base:end]
        # shrink using e_phoff if sane
        try:
            e_phoff = struct.unpack_from("<Q", data, base + 0x20)[0]
            e_phentsize = struct.unpack_from("<H", data, base + 0x36)[0]
            e_phnum = struct.unpack_from("<H", data, base + 0x38)[0]
            if e_phnum and e_phentsize == 56 and e_phoff < len(blob):
                max_end = 0
                for pi in range(e_phnum):
                    o = base + e_phoff + pi * e_phentsize
                    p_offset = struct.unpack_from("<Q", data, o + 8)[0]
                    p_filesz = struct.unpack_from("<Q", data, o + 32)[0]
                    max_end = max(max_end, p_offset + p_filesz)
                if 0x1000 < max_end < 0x2000000:
                    blob = data[base : base + max_end]
        except Exception:
            pass
        mz = movz_imm16(blob, 0x2F50)
        mz52 = movz_imm16(blob, 0x2F52)
        has_init = b"SIM_INIT_REQ" in blob
        has_sim_init = b"SIM_INIT" in blob
        has_ipc_tx = b"IpcTxSimInit" in blob
        oem_str = b"/dev/oem_ipc" in blob
        umts = b"/dev/umts_ipc" in blob
        lines.append(
            f"  elf@{base:#x} size={len(blob)} oem_str={oem_str} umts={umts} "
            f"SIM_INIT_REQ={has_init} SIM_INIT={has_sim_init} IpcTx={has_ipc_tx} "
            f"MOVZ#0x2f50={len(mz)} sample={[hex(base+x) for x in mz[:6]]} "
            f"MOVZ#0x2f52={len(mz52)} str_hit={h:#x}"
        )
        # dump a few distinctive strings
        for s in (b"SitOem", b"protobuf", b"SIM_", b"cbd", b"rild", b"ModemBoot", b"ramdump"):
            if s in blob:
                j = blob.find(s)
                lines.append(f"    has {s!r} @{base+j:#x}: {ctx(blob, j, 70)}")

    # Also: scan WHOLE vendor for MOVZ #0x2f50 and check nearby strings / oem
    lines.append("\n=== GLOBAL MOVZ #0x2f50 (sample + local OEM?) ===")
    all_mz = movz_imm16(data, 0x2F50)
    lines.append(f"count={len(all_mz)}")
    oem_near = 0
    for off in all_mz[:40]:
        window = data[max(0, off - 0x200) : off + 0x200]
        near_oem = b"oem_ipc" in window or b"OEM" in window or b"SIM_INIT" in window
        if near_oem:
            oem_near += 1
        if near_oem or off in all_mz[:8]:
            lines.append(f"  @{off:#x} near_oemish={near_oem} ctx={ctx(data, off, 48)}")
    lines.append(f"MOVZ sites with nearby oem/SIM_INIT window: {oem_near}/{min(40,len(all_mz))} sampled")

    # Hunt other ELF islands containing both umts_boot0 and SIM (classic cbd)
    # and any NEW island with oem_ipc + encode-like that we haven't carved
    lines.append("\n=== NEW CARVE CANDIDATES (oem_ipc + MOVZ 0x2f50) ===")
    found_encoder = False
    for base in sorted(seen_bases):
        # re-check with phdr size
        pass
    for h in oem_hits:
        base = find_elf_base(data, h)
        if base is None:
            continue
        nxt = data.find(b"\x7fELF", base + 0x1000)
        end = min(len(data), nxt if nxt > 0 else base + 0x400000)
        blob = data[base:end]
        mz = movz_imm16(blob, 0x2F50)
        if mz and b"/dev/oem_ipc" in blob:
            found_encoder = True
            lines.append(f"  CANDIDATE elf@{base:#x} size~={len(blob)} MOVZ@{[hex(base+x) for x in mz[:8]]}")
    if not found_encoder:
        lines.append("  NONE — no ELF island opens oem_ipc AND encodes MOVZ #0x2f50")

    # ASCII SIM_INIT_REQ absolute absence
    lines.append(f"\nSIM_INIT_REQ absolute count={len(needle_hits[b'SIM_INIT_REQ'])}")
    lines.append(f"IpcTxSimInit absolute count={len(needle_hits[b'IpcTxSimInit'])}")
    lines.append(f"SimInitMessage absolute count={len(needle_hits[b'SimInitMessage'])}")

    text = "\n".join(lines) + "\n"
    OUT.write_text(text, encoding="utf-8")
    print(text)
    print(f"WROTE {OUT}")


if __name__ == "__main__":
    main()
