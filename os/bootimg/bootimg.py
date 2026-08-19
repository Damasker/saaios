#!/usr/bin/env python3
"""Unpack / pack Android boot.img header v0–v2 (A12s stock is v2)."""
from __future__ import annotations

import argparse
import json
import struct
from pathlib import Path


def _u32(n: int) -> bytes:
    return struct.pack("<I", n)


def _u64(n: int) -> bytes:
    return struct.pack("<Q", n)


def parse_header(data: bytes) -> dict:
    if data[:8] != b"ANDROID!":
        raise ValueError("not an Android boot image")
    (
        kernel_size,
        kernel_addr,
        ramdisk_size,
        ramdisk_addr,
        second_size,
        second_addr,
        tags_addr,
        page_size,
        header_version,
        os_version,
    ) = struct.unpack_from("<10I", data, 8)
    name = data[48:64].split(b"\0", 1)[0].decode("ascii", "replace")
    cmdline = data[64:576].split(b"\0", 1)[0].decode("ascii", "replace")
    extra = data[608:1632].split(b"\0", 1)[0].decode("ascii", "replace")
    info = {
        "kernel_size": kernel_size,
        "kernel_addr": kernel_addr,
        "ramdisk_size": ramdisk_size,
        "ramdisk_addr": ramdisk_addr,
        "second_size": second_size,
        "second_addr": second_addr,
        "tags_addr": tags_addr,
        "page_size": page_size,
        "header_version": header_version,
        "os_version": os_version,
        "name": name,
        "cmdline": cmdline,
        "extra_cmdline": extra,
        "recovery_dtbo_size": 0,
        "recovery_dtbo_offset": 0,
        "header_size": 1660 if header_version >= 2 else 1648 if header_version >= 1 else 1632,
        "dtb_size": 0,
        "dtb_addr": 0,
    }
    if header_version >= 1:
        rec_size, rec_off, hdr_size = struct.unpack_from("<IQI", data, 1632)
        info["recovery_dtbo_size"] = rec_size
        info["recovery_dtbo_offset"] = rec_off
        info["header_size"] = hdr_size
    if header_version >= 2:
        dtb_size, dtb_addr = struct.unpack_from("<IQ", data, 1648)
        info["dtb_size"] = dtb_size
        info["dtb_addr"] = dtb_addr
    return info


def _align(n: int, page: int) -> int:
    return (n + page - 1) // page * page


def split_payload(data: bytes, info: dict) -> dict[str, bytes]:
    page = info["page_size"]
    pos = page
    kernel = data[pos : pos + info["kernel_size"]]
    pos += _align(info["kernel_size"], page)
    ramdisk = data[pos : pos + info["ramdisk_size"]]
    pos += _align(info["ramdisk_size"], page)
    second = b""
    if info["second_size"]:
        second = data[pos : pos + info["second_size"]]
        pos += _align(info["second_size"], page)
    rec = b""
    if info["header_version"] >= 1 and info["recovery_dtbo_size"]:
        rec = data[pos : pos + info["recovery_dtbo_size"]]
        pos += _align(info["recovery_dtbo_size"], page)
    dtb = b""
    if info["header_version"] >= 2 and info["dtb_size"]:
        dtb = data[pos : pos + info["dtb_size"]]
    return {"kernel": kernel, "ramdisk": ramdisk, "second": second, "recovery_dtbo": rec, "dtb": dtb}


def pack(info: dict, parts: dict[str, bytes]) -> bytes:
    page = info["page_size"]
    kernel = parts["kernel"]
    ramdisk = parts["ramdisk"]
    second = parts.get("second") or b""
    rec = parts.get("recovery_dtbo") or b""
    dtb = parts.get("dtb") or b""
    hdr_ver = info["header_version"]

    name = info.get("name", "").encode("ascii")[:16]
    cmdline = info.get("cmdline", "").encode("ascii")[:511]
    extra = info.get("extra_cmdline", "").encode("ascii")[:1023]

    hdr = bytearray(page)
    hdr[0:8] = b"ANDROID!"
    struct.pack_into(
        "<10I",
        hdr,
        8,
        len(kernel),
        info["kernel_addr"],
        len(ramdisk),
        info["ramdisk_addr"],
        len(second),
        info["second_addr"],
        info["tags_addr"],
        page,
        hdr_ver,
        info["os_version"],
    )
    hdr[48 : 48 + len(name)] = name
    hdr[64 : 64 + len(cmdline)] = cmdline
    hdr[608 : 608 + len(extra)] = extra
    if hdr_ver >= 1:
        rec_off = page + _align(len(kernel), page) + _align(len(ramdisk), page) + _align(len(second), page)
        struct.pack_into("<IQI", hdr, 1632, len(rec), rec_off if rec else 0, info.get("header_size", 1660))
    if hdr_ver >= 2:
        struct.pack_into("<IQ", hdr, 1648, len(dtb), info.get("dtb_addr", 0))

    def pad(blob: bytes) -> bytes:
        n = _align(len(blob), page)
        return blob + b"\x00" * (n - len(blob))

    img = bytes(hdr) + pad(kernel) + pad(ramdisk) + pad(second) + pad(rec) + pad(dtb)
    if info.get("seandroid"):
        # Samsung sboot looks for this immediately after the Android payload.
        # Magisk appends the same marker. Without it A12s hangs on the logo, then reboots.
        img += b"SEANDROIDENFORCE"
    pad_to = int(info.get("pad_to") or 0)
    if pad_to:
        if len(img) > pad_to:
            raise ValueError(f"boot image {len(img)} larger than pad_to {pad_to}")
        img += b"\x00" * (pad_to - len(img))
    return img


def cmd_unpack(args: argparse.Namespace) -> None:
    data = Path(args.boot).read_bytes()
    info = parse_header(data)
    parts = split_payload(data, info)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "header.json").write_text(json.dumps(info, indent=2) + "\n")
    (out / "kernel").write_bytes(parts["kernel"])
    (out / "ramdisk.cpio.gz").write_bytes(parts["ramdisk"])
    if parts["dtb"]:
        (out / "dtb").write_bytes(parts["dtb"])
    print(json.dumps(info, indent=2))
    print(f"kernel {len(parts['kernel'])} ramdisk {len(parts['ramdisk'])} dtb {len(parts['dtb'])}")


def cmd_pack(args: argparse.Namespace) -> None:
    src = Path(args.src)
    info = json.loads((src / "header.json").read_text())
    if args.cmdline:
        info["cmdline"] = args.cmdline
    info["seandroid"] = bool(args.seandroid)
    info["pad_to"] = args.pad_to or 0
    ramdisk_path = Path(args.ramdisk) if args.ramdisk else src / "ramdisk.cpio.gz"
    parts = {
        "kernel": (src / "kernel").read_bytes(),
        "ramdisk": ramdisk_path.read_bytes(),
        "second": b"",
        "recovery_dtbo": b"",
        "dtb": (src / "dtb").read_bytes() if (src / "dtb").exists() else b"",
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(pack(info, parts))
    print(f"wrote {out} ({out.stat().st_size} bytes)")


def main() -> None:
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="cmd", required=True)
    u = sub.add_parser("unpack")
    u.add_argument("boot")
    u.add_argument("-o", "--out", required=True)
    u.set_defaults(func=cmd_unpack)
    k = sub.add_parser("pack")
    k.add_argument("src")
    k.add_argument("-o", "--out", required=True)
    k.add_argument("--ramdisk")
    k.add_argument("--cmdline")
    k.add_argument("--seandroid", action="store_true", help="append SEANDROIDENFORCE (Samsung sboot)")
    k.add_argument("--pad-to", type=int, default=0, help="zero-pad to partition size so Odin does not leave old AVB tail")
    k.set_defaults(func=cmd_pack)
    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
