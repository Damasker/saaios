#!/usr/bin/env python3
"""Host-side SM-A127F UP_PARAM packer. Never flashes.

Stock BL (A127FXXSDDXJ2) ships `up_param.bin.lz4`. Decompressed, that is a
GNU/ustar of JPEGs that sboot blits *before* the kernel. Two different
Samsung pictures live here:

  Unlock warning (this tool's target)
      svb_orange.jpg        — orange state, "PRESS POWER KEY TO CONTINUE"
      booting_warning.jpg   — "not running Samsung's official software"

  Boot splash (also blanked in the nologo tar; DECON leftover until fb0)
      logo.jpg              — SAMSUNG / Galaxy A12 / Knox
      letter.jpg            — SAMSUNG wordmark

Download-mode / OEM-unlock teal screens stay stock. A bad PARAM can replace
the cyan "Downloading…" splash with garbage; we do not touch those JPEGs.

Odin: put the resulting tar in the **BL** slot. Inner name must stay
`up_param.bin.lz4`. Do not add sboot/tz/ldfw/keystorage/tzar/vbmeta.
Human flashes. `flash` / `make flash` refuse.
"""
from __future__ import annotations

import argparse
import hashlib
import io
import shutil
import struct
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

STOCK_BL_NAME = (
    "BL_A127FXXSDDXJ2_QB86601342_REV00_user_low_ship_MULTI_CERT.tar.md5.zip"
)
INNER_TAR_MD5 = (
    "BL_A127FXXSDDXJ2_QB86601342_REV00_user_low_ship_MULTI_CERT.tar.md5"
)
UP_PARAM_LZ4 = "up_param.bin.lz4"
UP_PARAM_BIN = "up_param.bin"

# Never pack these into an Odin tar (would be a BL/TZ flash).
FORBIDDEN_ODIN_NAMES = {
    "sboot.bin",
    "sboot.bin.lz4",
    "tzsw.img",
    "tzsw.img.lz4",
    "tzar.img",
    "tzar.img.lz4",
    "ldfw.img",
    "ldfw.img.lz4",
    "keystorage.bin",
    "keystorage.bin.lz4",
    "vbmeta.img",
    "vbmeta.img.lz4",
    "param.bin",
    "param.bin.lz4",
}

# Blanked in nologo. Same pixel size as stock (not 1×1) so sboot blit stays
# full-rect black instead of a dot on leftover framebuffer.
BLANK_JPEGS = (
    "svb_orange.jpg",
    "booting_warning.jpg",
    "logo.jpg",
    "letter.jpg",
)

# Teal Download-mode / OEM-unlock / error art — keep stock.
KEEP_JPEGS_NOTE = (
    "download.jpg",
    "download_error.jpg",
    "warning.jpg",
    "warning_svb.jpg",
    "device_unlock.jpg",
    "device_lock.jpg",
    "broken_cable.jpg",
    "secure_error.jpg",
    "lpm.jpg",
    "low_battery_alert.jpg",
)

MEDIA_NOLOGO = "saaios-up_param-nologo.tar"
MEDIA_RESTORE = "saaios-up_param-stock-restore.tar"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def jpeg_info(data: bytes) -> tuple[int, int, int]:
    """Return (width, height, ncomp) from SOF."""
    if data[:2] != b"\xff\xd8":
        raise ValueError("not a JPEG")
    i = 2
    while i + 4 <= len(data):
        if data[i] != 0xFF:
            i += 1
            continue
        marker = data[i + 1]
        i += 2
        if marker in (0xD8, 0xD9, 0x01) or 0xD0 <= marker <= 0xD7:
            continue
        if i + 2 > len(data):
            break
        seglen = struct.unpack(">H", data[i : i + 2])[0]
        if marker in (
            0xC0,
            0xC1,
            0xC2,
            0xC3,
            0xC5,
            0xC6,
            0xC7,
            0xC9,
            0xCA,
            0xCB,
            0xCD,
            0xCE,
            0xCF,
        ):
            h, w = struct.unpack(">HH", data[i + 3 : i + 7])
            ncomp = data[i + 7]
            return w, h, ncomp
        i += seglen
    raise ValueError("JPEG has no SOF")


def jpeg_size(data: bytes) -> tuple[int, int]:
    w, h, _n = jpeg_info(data)
    return w, h


def black_jpeg(width: int, height: int) -> bytes:
    """Baseline 8-bit JPEG, solid black, same WxH as the stock splash."""
    convert = shutil.which("convert") or shutil.which("magick")
    if not convert:
        raise RuntimeError("ImageMagick convert/magick required")
    cmd = [
        convert,
        "-size",
        f"{width}x{height}",
        "xc:black",
        "-type",
        "TrueColor",
        "-sampling-factor",
        "4:2:0",
        "-quality",
        "80",
        "-interlace",
        "none",
        "jpeg:-",
    ]
    if Path(convert).name == "magick":
        cmd.insert(1, "convert")
    data = subprocess.check_output(cmd)
    w, h, ncomp = jpeg_info(data)
    if (w, h) != (width, height):
        raise RuntimeError(f"black jpeg {w}x{h} != requested {width}x{height}")
    if ncomp != 3:
        raise RuntimeError(f"black jpeg must be 3-component YCbCr, got ncomp={ncomp}")
    return data


def lz4_compress(src: Path, dst: Path) -> None:
    """Match stock BL frame: 1 MiB blocks + content size (magic 04 22 4d 18)."""
    lz4 = shutil.which("lz4")
    if not lz4:
        raise RuntimeError("lz4 required")
    subprocess.check_call(
        [lz4, "-f", "-B6", "--content-size", "-q", str(src), str(dst)]
    )


def lz4_decompress(src: Path, dst: Path) -> None:
    lz4 = shutil.which("lz4")
    if not lz4:
        raise RuntimeError("lz4 required")
    subprocess.check_call([lz4, "-d", "-f", "-q", str(src), str(dst)])


def extract_up_param_lz4(bl_zip: Path, dest: Path) -> Path:
    dest.mkdir(parents=True, exist_ok=True)
    out_lz4 = dest / UP_PARAM_LZ4
    with zipfile.ZipFile(bl_zip) as z:
        names = z.namelist()
        if INNER_TAR_MD5 not in names:
            raise FileNotFoundError(f"{INNER_TAR_MD5} not in {bl_zip}")
        raw = z.read(INNER_TAR_MD5)
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:") as t:
        member = t.getmember(UP_PARAM_LZ4)
        extracted = t.extractfile(member)
        if extracted is None:
            raise RuntimeError("up_param.bin.lz4 missing in BL tar")
        out_lz4.write_bytes(extracted.read())
    return out_lz4


def unpack_up_param_tar(up_param: Path) -> list[tuple[tarfile.TarInfo, bytes]]:
    members: list[tuple[tarfile.TarInfo, bytes]] = []
    with tarfile.open(up_param, mode="r:") as t:
        for info in t.getmembers():
            if not info.isfile():
                continue
            f = t.extractfile(info)
            if f is None:
                continue
            members.append((info, f.read()))
    if not members:
        raise RuntimeError(f"{up_param} has no files")
    return members


def pack_up_param_tar(members: list[tuple[tarfile.TarInfo, bytes]], dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(dest, mode="w:", format=tarfile.USTAR_FORMAT) as t:
        for info, data in members:
            cloned = tarfile.TarInfo(name=info.name)
            cloned.size = len(data)
            cloned.mode = info.mode
            cloned.uid = info.uid
            cloned.gid = info.gid
            cloned.uname = info.uname
            cloned.gname = info.gname
            cloned.mtime = info.mtime
            cloned.type = tarfile.REGTYPE
            t.addfile(cloned, io.BytesIO(data))


def assert_odin_tar_safe(tar_path: Path) -> None:
    with tarfile.open(tar_path, mode="r:") as t:
        names = [m.name for m in t.getmembers() if m.isfile()]
    bad = [n for n in names if Path(n).name in FORBIDDEN_ODIN_NAMES]
    if bad:
        raise RuntimeError(f"refusing Odin tar with BL/TZ/EFS pieces: {bad}")
    if names != [UP_PARAM_LZ4]:
        raise RuntimeError(f"Odin tar must contain only {UP_PARAM_LZ4}, got {names}")


def write_odin_tar(lz4_path: Path, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(dest, mode="w:") as t:
        t.add(lz4_path, arcname=UP_PARAM_LZ4)
    assert_odin_tar_safe(dest)


def cmd_flash(_args: argparse.Namespace) -> None:
    print("REFUSING: will not flash PARAM / UP_PARAM / BL / sboot / TZ / EFS.", file=sys.stderr)
    print("Human: Download mode, Odin BL slot = saaios-up_param-nologo.tar", file=sys.stderr)
    print("Rollback: Odin BL slot = saaios-up_param-stock-restore.tar", file=sys.stderr)
    print("Do not flash sboot, tzsw, tzar, ldfw, keystorage, EFS, boot, vbmeta.", file=sys.stderr)
    raise SystemExit(2)


def cmd_extract(args: argparse.Namespace) -> None:
    stock = Path(args.stock)
    bl_zip = Path(args.bl_zip)
    lz4 = extract_up_param_lz4(bl_zip, stock)
    bin_path = stock / UP_PARAM_BIN
    lz4_decompress(lz4, bin_path)
    members = unpack_up_param_tar(bin_path)
    print(f"stock {lz4} {lz4.stat().st_size} bytes sha256={sha256_file(lz4)}")
    print(f"stock {bin_path} {bin_path.stat().st_size} bytes sha256={sha256_file(bin_path)}")
    print(f"{len(members)} JPEG members:")
    for info, data in members:
        try:
            w, h = jpeg_size(data)
            geo = f"{w}x{h}"
        except ValueError:
            geo = "?"
        mark = "BLANK" if info.name in BLANK_JPEGS else "keep"
        print(f"  {info.name:24} {len(data):7d}  {geo:10} {mark}")


def cmd_build(args: argparse.Namespace) -> None:
    stock = Path(args.stock)
    out = Path(args.out)
    media = Path(args.media) if args.media else None
    bl_zip = Path(args.bl_zip)

    if not (stock / UP_PARAM_LZ4).is_file() or not (stock / UP_PARAM_BIN).is_file():
        cmd_extract(args)

    stock_bin = stock / UP_PARAM_BIN
    stock_lz4 = stock / UP_PARAM_LZ4
    members = unpack_up_param_tar(stock_bin)
    names = [m.name for m, _ in members]
    missing = [n for n in BLANK_JPEGS if n not in names]
    if missing:
        raise RuntimeError(f"stock up_param missing {missing}")

    new_members: list[tuple[tarfile.TarInfo, bytes]] = []
    replaced: list[str] = []
    for info, data in members:
        if info.name in BLANK_JPEGS:
            w, h = jpeg_size(data)
            data = black_jpeg(w, h)
            replaced.append(f"{info.name} ({w}x{h} black, {len(data)} bytes)")
        new_members.append((info, data))

    work = Path(tempfile.mkdtemp(prefix="saaios-up_param-"))
    try:
        nologo_bin = work / UP_PARAM_BIN
        nologo_lz4 = work / UP_PARAM_LZ4
        pack_up_param_tar(new_members, nologo_bin)
        if nologo_bin.stat().st_size > 8 * 1024 * 1024:
            raise RuntimeError("up_param.bin larger than 8 MiB UP_PARAM partition")
        lz4_compress(nologo_bin, nologo_lz4)

        out.mkdir(parents=True, exist_ok=True)
        nologo_dir = out / "odin-up_param-nologo"
        restore_dir = out / "odin-up_param-stock-restore"
        shutil.rmtree(nologo_dir, ignore_errors=True)
        shutil.rmtree(restore_dir, ignore_errors=True)
        nologo_dir.mkdir()
        restore_dir.mkdir()
        shutil.copy2(nologo_lz4, nologo_dir / UP_PARAM_LZ4)
        shutil.copy2(nologo_bin, out / "up_param-nologo.bin")
        shutil.copy2(stock_lz4, restore_dir / UP_PARAM_LZ4)

        nologo_tar = out / MEDIA_NOLOGO
        restore_tar = out / MEDIA_RESTORE
        write_odin_tar(nologo_dir / UP_PARAM_LZ4, nologo_tar)
        write_odin_tar(restore_dir / UP_PARAM_LZ4, restore_tar)

        # Bit-identical restore: inner lz4 must match stock BL.
        if sha256_file(restore_dir / UP_PARAM_LZ4) != sha256_file(stock_lz4):
            raise RuntimeError("restore lz4 does not match stock BL")

        copied = []
        if media is not None:
            media.mkdir(parents=True, exist_ok=True)
            for src in (nologo_tar, restore_tar):
                dst = media / src.name
                shutil.copy2(src, dst)
                copied.append(dst)

        print("built (not flashed):")
        print(f"  {nologo_tar}  sha256={sha256_file(nologo_tar)}")
        print(f"  {restore_tar}  sha256={sha256_file(restore_tar)}")
        print(f"  inner nologo {UP_PARAM_BIN} {nologo_bin.stat().st_size} bytes")
        print("replaced with same-size black JPEG:")
        for line in replaced:
            print(f"  {line}")
        print("kept (Download-mode / errors / SUD / lock art):")
        for n in names:
            if n not in BLANK_JPEGS:
                print(f"  {n}")
        print("Odin slot: BL  (filename up_param.bin.lz4 only; not AP, not full stock BL)")
        if copied:
            print("copied to Samba share:")
            for p in copied:
                print(f"  {p}  sha256={sha256_file(p)}")
                print(f"  \\\\192.168.168.110\\media\\{p.name}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--stock",
        default="os/images/stock/A127FXXSDDXJ2",
        help="extracted stock dir (gitignored)",
    )
    p.add_argument(
        "--bl-zip",
        default=f"/srv/media/{STOCK_BL_NAME}",
        help="stock BL zip on the media share",
    )
    p.add_argument("--out", default="os/build", help="host build dir")
    p.add_argument("--media", default="/srv/media", help="Samba copy dest; empty to skip")
    sub = p.add_subparsers(dest="cmd", required=True)
    e = sub.add_parser("extract", help="unpack stock up_param from BL zip")
    e.set_defaults(func=cmd_extract)
    b = sub.add_parser("build", help="nologo + stock-restore Odin tars (no flash)")
    b.set_defaults(func=cmd_build)
    f = sub.add_parser("flash", help="refuse to flash")
    f.set_defaults(func=cmd_flash)
    args = p.parse_args()
    if args.cmd == "build" and args.media == "":
        args.media = None
    args.func(args)


if __name__ == "__main__":
    main()
