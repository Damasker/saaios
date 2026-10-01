#!/usr/bin/env python3
"""Vendor.img + full libsitril deep 0x2f50 const-build (beyond carved stubs).

Scans MOVZ/MOVK/MOVN/ORR and reports ELF islands that also touch oem_ipc.
"""
from __future__ import annotations

import struct
from pathlib import Path

VENDOR = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/"
    "cdma-hunt/factory-td1a-vendor/vendor.img"
)
LIBSIT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "tmp-vendor-deep-2f50-constbuild-vendor.out"
)
IMM = 0x2F50
INV = (~IMM) & 0xFFFF  # 0xd0af


def find_all(data: bytes, needle: bytes, align: int = 1, limit: int = 500) -> list[int]:
    out, start = [], 0
    while len(out) < limit:
        i = data.find(needle, start)
        if i < 0:
            break
        if align == 1 or i % align == 0:
            out.append(i)
        start = i + 1
    return out


def mov_hits(data: bytes, opc: int, imm: int) -> list[tuple[int, int]]:
    hits = []
    base = opc | ((imm & 0xFFFF) << 5)
    for rd in range(32):
        w = struct.pack("<I", base | rd)
        for off in find_all(data, w, 4, 500):
            hits.append((off, rd))
    return sorted(hits)


def decode_bitmask(N: int, immr: int, imms: int, regsize: int) -> int:
    if N == 1:
        length = 6
    else:
        t = (~imms) & 0x3F
        if t == 0:
            raise ValueError("bad")
        length = t.bit_length() - 1
        if length < 1:
            raise ValueError("bad")
    esize = 1 << length
    if esize > regsize:
        raise ValueError("esize")
    levels = esize - 1
    S = imms & levels
    R = immr & levels
    if S == levels:
        raise ValueError("all1")
    welem = (1 << (S + 1)) - 1
    welem = ((welem >> R) | (welem << (esize - R))) & ((1 << esize) - 1)
    out = 0
    for i in range(0, regsize, esize):
        out |= welem << i
    return out


def orr_imm_val(insn: int) -> int | None:
    if ((insn >> 29) & 3) != 0b01:
        return None
    if ((insn >> 23) & 0x3F) != 0b100100:
        return None
    sf = (insn >> 31) & 1
    N = (insn >> 22) & 1
    immr = (insn >> 16) & 0x3F
    imms = (insn >> 10) & 0x3F
    rn = (insn >> 5) & 0x1F
    if rn != 31:
        return None
    try:
        val = decode_bitmask(N, immr, imms, 64 if sf else 32)
    except ValueError:
        return None
    return val & (0xFFFFFFFFFFFFFFFF if sf else 0xFFFFFFFF)


def elf_base(data: bytes, hit: int, back: int = 0x800000) -> int | None:
    start = max(0, hit - back)
    i = hit
    while i >= start:
        i = data.rfind(b"\x7fELF", start, i + 1)
        if i < 0:
            return None
        if data[i + 4] == 2 and struct.unpack_from("<H", data, i + 16)[0] in (2, 3):
            return i
    return None


def island_flags(data: bytes, base: int | None) -> str:
    if base is None:
        return "no_elf"
    nxt = data.find(b"\x7fELF", base + 0x1000)
    end = min(len(data), nxt if nxt > 0 else base + 0x400000)
    blob = data[base:end]
    return (
        f"sz~{len(blob)} oem={b'/dev/oem_ipc' in blob} umts={b'/dev/umts_ipc' in blob} "
        f"SIM_INIT_REQ={b'SIM_INIT_REQ' in blob} SIM_INIT={b'SIM_INIT' in blob} "
        f"SitOem={b'SitOem' in blob} BuildOemSim={b'BuildOemSimRequest' in blob}"
    )


def analyze_blob(name: str, data: bytes, lines: list[str]) -> None:
    lines.append(f"\n=== {name} size={len(data)} ===")
    mz_w = mov_hits(data, 0x52800000, IMM)
    mz_x = mov_hits(data, 0xD2800000, IMM)
    movn = mov_hits(data, 0x12800000, INV) + mov_hits(data, 0x92800000, INV)
    lines.append(f"MOVZ_W={len(mz_w)} MOVZ_X={len(mz_x)} MOVN_~={len(movn)}")

    # MOVZ+MOVK same rd within +/-32 insn; report exact 0x2f50 and pointer-ish
    exact_pairs = 0
    ptr_pairs = 0
    sample_exact = []
    for off, rd in mz_w + mz_x:
        win_base = max(0, off - 128)
        win = data[win_base : off + 128]
        for rel in range(0, len(win) - 3, 4):
            insn = struct.unpack_from("<I", win, rel)[0]
            abs_off = win_base + rel
            if abs_off == off:
                continue
            # MOVK any
            top = insn & 0xFF800000
            if top not in (
                0x72800000,
                0x72A00000,
                0x72C00000,
                0x72E00000,
                0xF2800000,
                0xF2A00000,
                0xF2C00000,
                0xF2E00000,
            ):
                continue
            if (insn & 0x1F) != rd:
                continue
            imm16 = (insn >> 5) & 0xFFFF
            hw = (insn >> 21) & 3
            built = IMM | (imm16 << (16 * hw))
            if built == IMM:
                exact_pairs += 1
                if len(sample_exact) < 8:
                    sample_exact.append((off, abs_off, rd, built))
            else:
                ptr_pairs += 1
    lines.append(f"MOVZ+MOVK exact0x2f50={exact_pairs} pointerish={ptr_pairs}")
    for a, b, rd, built in sample_exact:
        lines.append(f"  exact movz@{a:#x} movk@{b:#x} rd={rd} built={built:#x} {island_flags(data, elf_base(data,a))}")

    # ORR scan — only full file if < 8MB; else sample around MOVZ sites only for vendor
    orr_hits = []
    if len(data) <= 8 * 1024 * 1024:
        mv = memoryview(data).cast("I")
        for i, insn in enumerate(mv):
            val = orr_imm_val(int(insn))
            if val is None:
                continue
            if (val & 0xFFFFFFFF) == IMM or (val & 0xFFFF) == IMM:
                orr_hits.append((i * 4, val))
    else:
        # vendor: scan 512B windows around every LE 50 2f aligned? too many.
        # Instead decode every insn in +/-256 of each MOVZ site (already covered)
        # Plus scan ORR by walking all 4-aligned words is too slow on 665MB —
        # use bookmark: search cannot find ORR encoding easily; sample every 4MB chunk of .text-ish
        # Fast path: only check windows around keyword strings
        seeds = []
        for n in (b"/dev/oem_ipc", b"SIM_INIT_REQ", b"oem_ipc_message", b"IpcTxSimInit"):
            seeds.extend(find_all(data, n, 1, 40))
        for s in seeds:
            a = max(0, s - 0x2000) & ~3
            b = min(len(data), s + 0x2000)
            for off in range(a, b - 3, 4):
                insn = struct.unpack_from("<I", data, off)[0]
                val = orr_imm_val(insn)
                if val is None:
                    continue
                if (val & 0xFFFFFFFF) == IMM or (val & 0xFFFF) == IMM:
                    orr_hits.append((off, val))
    lines.append(f"ORR_imm hits={len(orr_hits)}")
    for off, val in orr_hits[:12]:
        lines.append(f"  orr@{off:#x} val={val:#x} {island_flags(data, elf_base(data, off))}")

    # Classify each MOVZ site
    lines.append("MOVZ site islands:")
    seen = set()
    for off, rd in (mz_w + mz_x)[:40]:
        base = elf_base(data, off)
        key = (base, off)
        if key in seen:
            continue
        seen.add(key)
        lines.append(f"  movz@{off:#x} rd={rd} base={base and hex(base)} {island_flags(data, base)}")

    for off, rd in movn[:10]:
        lines.append(f"  movn@{off:#x} rd={rd} base={elf_base(data,off) and hex(elf_base(data,off))} {island_flags(data, elf_base(data,off))}")


def main() -> None:
    lines = ["Deep vendor/libsitril 0x2f50 const-build extension"]
    if LIBSIT.exists():
        analyze_blob(str(LIBSIT), LIBSIT.read_bytes(), lines)
    else:
        lines.append(f"MISSING {LIBSIT}")
    # vendor — streaming read
    lines.append(f"\nloading vendor {VENDOR} ...")
    data = VENDOR.read_bytes()
    analyze_blob(str(VENDOR), data, lines)

    # Cross-check: any island with oem_ipc AND (MOVZ 0x2f50 or ORR)
    lines.append("\n=== CROSS: oem_ipc string islands vs 0x2f50 MOVZ ===")
    oem_hits = find_all(data, b"/dev/oem_ipc", 1, 50)
    bases = []
    for h in oem_hits:
        b = elf_base(data, h)
        if b is not None and b not in bases:
            bases.append(b)
    mz_all = set(o for o, _ in mov_hits(data, 0x52800000, IMM) + mov_hits(data, 0xD2800000, IMM))
    for b in bases:
        nxt = data.find(b"\x7fELF", b + 0x1000)
        end = min(len(data), nxt if nxt > 0 else b + 0x400000)
        mz_in = sorted(o for o in mz_all if b <= o < end)
        lines.append(
            f"oem_elf@{b:#x} end~{end:#x} MOVZ_2f50_in_island={len(mz_in)} "
            f"offs={[hex(x) for x in mz_in[:6]]} flags={island_flags(data,b)}"
        )

    text = "\n".join(lines) + "\n"
    OUT.write_text(text, encoding="utf-8")
    print(text)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
