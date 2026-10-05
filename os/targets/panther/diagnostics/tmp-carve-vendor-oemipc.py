#!/usr/bin/env python3
"""Carve ELF(s) from vendor.img that reference /dev/oem_ipc*; RE write format."""
from __future__ import annotations

import struct
from pathlib import Path

VENDOR = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img"
)
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor"
)
data = VENDOR.read_bytes()
print(f"vendor.img size={len(data)}")


def find_elf_start(pos: int, back: int = 64 * 1024 * 1024) -> int | None:
    """Walk back for ELF magic; prefer PT_LOAD-looking headers."""
    lo = max(0, pos - back)
    # search backwards for \\x7fELF
    i = pos
    while i >= lo:
        j = data.rfind(b"\x7fELF", lo, i + 1)
        if j < 0:
            return None
        # basic sanity: 64-bit LE, ET_DYN/ET_EXEC
        if j + 0x40 <= len(data):
            ei_class = data[j + 4]
            ei_data = data[j + 5]
            e_type = struct.unpack_from("<H", data, j + 16)[0]
            e_machine = struct.unpack_from("<H", data, j + 18)[0]
            if ei_class == 2 and ei_data == 1 and e_machine == 0xB7 and e_type in (2, 3):
                return j
        i = j - 1
    return None


def elf_size_guess(elf_off: int) -> int:
    """Estimate ELF file size from program/section headers."""
    e_phoff = struct.unpack_from("<Q", data, elf_off + 32)[0]
    e_shoff = struct.unpack_from("<Q", data, elf_off + 40)[0]
    e_phentsize = struct.unpack_from("<H", data, elf_off + 54)[0]
    e_phnum = struct.unpack_from("<H", data, elf_off + 56)[0]
    e_shentsize = struct.unpack_from("<H", data, elf_off + 58)[0]
    e_shnum = struct.unpack_from("<H", data, elf_off + 60)[0]
    end = elf_off + 64
    # phdrs
    if e_phoff and e_phnum and e_phentsize:
        ph_end = elf_off + e_phoff + e_phnum * e_phentsize
        end = max(end, ph_end)
        for i in range(e_phnum):
            off = elf_off + e_phoff + i * e_phentsize
            if off + 56 > len(data):
                break
            p_offset = struct.unpack_from("<Q", data, off + 8)[0]
            p_filesz = struct.unpack_from("<Q", data, off + 32)[0]
            end = max(end, elf_off + p_offset + p_filesz)
    if e_shoff and e_shnum and e_shentsize:
        sh_end = elf_off + e_shoff + e_shnum * e_shentsize
        end = max(end, sh_end)
        for i in range(min(e_shnum, 256)):
            off = elf_off + e_shoff + i * e_shentsize
            if off + 64 > len(data):
                break
            sh_offset = struct.unpack_from("<Q", data, off + 24)[0]
            sh_size = struct.unpack_from("<Q", data, off + 32)[0]
            if sh_size < 0x20000000:
                end = max(end, elf_off + sh_offset + sh_size)
    return end - elf_off


hits = []
for needle in (b"/dev/oem_ipc0", b"/dev/oem_ipc1", b"/dev/oem_ipc"):
    pos = 0
    while True:
        i = data.find(needle, pos)
        if i < 0:
            break
        hits.append((i, needle.decode()))
        pos = i + 1

print("string hits:")
for off, n in hits:
    print(f"  {off} {n}")

carved = {}
for off, n in hits:
    elf = find_elf_start(off)
    if elf is None:
        print(f"NO ELF for {n} @{off}")
        continue
    sz = elf_size_guess(elf)
    print(f"{n} @{off} -> ELF @{elf} size_guess={sz}")
    key = elf
    carved.setdefault(key, {"size": sz, "strings": []})
    carved[key]["strings"].append((off - elf, n))
    carved[key]["size"] = max(carved[key]["size"], sz, (off - elf) + 0x1000)

for elf_off, meta in carved.items():
    sz = min(meta["size"], 32 * 1024 * 1024)  # cap 32MB carve
    # bump size to cover farthest string
    for soff, _ in meta["strings"]:
        sz = max(sz, soff + 0x10000)
    sz = min(sz, len(data) - elf_off, 64 * 1024 * 1024)
    blob = data[elf_off : elf_off + sz]
    # try get SONAME / path hint from dynamic
    out = OUT / f"carved-oemipc-{elf_off:x}.so"
    out.write_bytes(blob)
    print(f"WROTE {out} bytes={len(blob)} strings={meta['strings']}")

    # Analyze write format
    b = blob
    print(f"\n=== analyze {out.name} ===")
    for nd in (
        b"/dev/oem_ipc0",
        b"/dev/oem_ipc1",
        b"/dev/oem_ipc",
        b"protobufSerialDataLen",
        b"SIM_INIT",
        b"2f50",
        b"OemIpc",
        b"write",
        b"Token %d",
    ):
        c = b.count(nd) if len(nd) > 3 else 0
        i = b.find(nd)
        print(f"  {nd!r} count~={b.count(nd)} first={hex(i) if i>=0 else -1}")

    # AArch64 MOVZ #0x2f50
    movz_hits = []
    for i in range(0, len(b) - 4, 4):
        w = struct.unpack_from("<I", b, i)[0]
        if (w & 0xFF800000) == 0x52800000:
            imm = (w >> 5) & 0xFFFF
            rd = w & 0x1F
            if imm in (0x2F50, 0x2F52, 0x2F57, 0x2F58, 0x200, 0x201):
                movz_hits.append((i, hex(imm), rd))
    print(f"  MOVZ interesting: {movz_hits[:20]}")

    # Look near /dev/oem_ipc0 for open/write patterns — dump ascii neighborhood
    for nd in (b"/dev/oem_ipc0", b"/dev/oem_ipc1", b"protobufSerialDataLen"):
        i = b.find(nd)
        if i < 0:
            continue
        ctx = bytes(x if 32 <= x < 127 else 0x2E for x in b[max(0, i - 200) : i + 200])
        print(f"  ctx {nd!r}:\n    {ctx.decode()}")

print("DONE")
