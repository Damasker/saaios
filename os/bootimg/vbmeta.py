#!/usr/bin/env python3
"""Build a vbmeta image with AVB verification disabled (algorithm NONE)."""
from __future__ import annotations

import argparse
from pathlib import Path
import struct

AVB_MAGIC = b"AVB0"
HEADER_SIZE = 256
# libavb: HASHTREE_DISABLED | VERIFICATION_DISABLED
FLAGS_DISABLED = 3


def patch_stock(stock: bytes, flags: int = FLAGS_DISABLED) -> bytes:
    """Magisk-style: keep Samsung vbmeta, only OR disable flags into the header."""
    if stock[:4] != AVB_MAGIC:
        raise ValueError("not a vbmeta image")
    if len(stock) < HEADER_SIZE:
        raise ValueError("vbmeta too small")
    out = bytearray(stock)
    current = struct.unpack_from(">I", out, 120)[0]
    struct.pack_into(">I", out, 120, current | flags)
    return bytes(out)


def make_vbmeta(flags: int = FLAGS_DISABLED, pad_to: int = 4096) -> bytes:
    hdr = bytearray(HEADER_SIZE)
    hdr[0:4] = AVB_MAGIC
    struct.pack_into(">I", hdr, 4, 1)  # required_libavb_version_major
    struct.pack_into(">I", hdr, 8, 0)  # required_libavb_version_minor
    # authentication/auxiliary sizes stay 0; algorithm_type NONE (0)
    struct.pack_into(">I", hdr, 120, flags)
    rel = b"avbtool 1.2.0"
    hdr[128 : 128 + len(rel)] = rel
    if pad_to < HEADER_SIZE:
        raise ValueError("pad_to smaller than vbmeta header")
    return bytes(hdr) + b"\x00" * (pad_to - HEADER_SIZE)


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("-o", "--out", required=True)
    p.add_argument("--flags", type=int, default=FLAGS_DISABLED)
    p.add_argument("--pad-to", type=int, default=4096)
    p.add_argument("--patch-stock", help="stock vbmeta.img to patch in place (same size)")
    args = p.parse_args()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    if args.patch_stock:
        data = patch_stock(Path(args.patch_stock).read_bytes(), flags=args.flags)
    else:
        data = make_vbmeta(flags=args.flags, pad_to=args.pad_to)
    out.write_bytes(data)
    flags = struct.unpack_from(">I", data, 120)[0]
    print(f"wrote {out} ({len(data)} bytes, flags={flags})")


if __name__ == "__main__":
    main()
