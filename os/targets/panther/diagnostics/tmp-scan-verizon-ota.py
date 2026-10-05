#!/usr/bin/env python3
"""Stream-scan Verizon OTA without loading whole zip into RAM."""
from __future__ import annotations

import hashlib
import lzma
import struct
import zipfile
from pathlib import Path

ZIP = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/panther-ota-td1a.221105.003-32ef0dee.zip"
)
EXPECT_SHA = "32ef0dee678d2b2b70396eb5538595c29b8d65fb6282ac52f0f53b55848f1d83"
OUT_DIR = ZIP.parent
NEEDLES = (
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"g5300q-",
    b"g5300g-",
    b"Found CDMA",
    b"SupportedRatMap",
)


def sha256_file(path: Path, chunk: int = 8 * 1024 * 1024) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def count_needles_file(path: Path, chunk: int = 8 * 1024 * 1024) -> dict:
    counts = {n: 0 for n in NEEDLES}
    overlap = max(len(n) for n in NEEDLES) - 1
    prev = b""
    with path.open("rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            window = prev + b
            for n in NEEDLES:
                counts[n] += window.count(n)
            prev = window[-overlap:] if overlap else b""
    return counts


def scan_bytes(data: bytes, label: str) -> dict:
    h = hashlib.sha256(data).hexdigest()
    no = data.count(b"No CDMA in SupportedRatMap")
    en = data.count(b"EnableCdmaRat")
    gq = data.count(b"g5300q-")
    gg = data.count(b"g5300g-")
    fc = data.count(b"Found CDMA")
    i = data.find(b"g5300q-") if gq else data.find(b"g5300g-")
    ver = data[i : i + 56].split(b"\0", 1)[0] if i >= 0 else b""
    print(
        f"SCAN {label}: sha={h} size={len(data)} "
        f"g5300q={gq} g5300g={gg} no_cdma={no} enable={en} "
        f"found_cdma={fc} ver={ver!r}"
    )
    return {
        "sha": h,
        "g5300q": gq,
        "g5300g": gg,
        "no_cdma": no,
        "enable": en,
        "interesting": gq > 0 and no == 0,
        "data": data if (gq or gg) else None,
    }


def extract_payload_member(zpath: Path) -> Path | None:
    """Extract payload.bin to disk if present; return path."""
    out = OUT_DIR / "tmp-td1a-payload.bin"
    with zipfile.ZipFile(zpath, "r") as z:
        names = z.namelist()
        print(f"members={len(names)} sample={names[:25]}")
        for n in names:
            low = n.lower()
            if low.endswith(("radio.img", "modem.img")) or (
                "radio" in low and low.endswith(".img")
            ):
                print(f"direct radio member {n}")
                data = z.read(n)
                r = scan_bytes(data, n)
                if r["interesting"]:
                    staged = OUT_DIR / f"STAGED-verizon-td1a221105003-{r['sha'][:12]}.img"
                    staged.write_bytes(data)
                    print("STAGED", staged)
                return None
        if "payload.bin" not in names:
            print("no payload.bin")
            return None
        print("extracting payload.bin to disk...")
        with z.open("payload.bin") as src, out.open("wb") as dst:
            while True:
                chunk = src.read(8 * 1024 * 1024)
                if not chunk:
                    break
                dst.write(chunk)
        print(f"payload size={out.stat().st_size}")
        return out


def hunt_xz_in_payload(payload_path: Path) -> bytes | None:
    """Scan payload for xz streams containing g5300 / RatMap."""
    # header
    with payload_path.open("rb") as f:
        hdr = f.read(64)
    if hdr[:4] != b"CrAU":
        print("not CrAU", hdr[:8])
        return None
    version, manifest_size = struct.unpack(">QQ", hdr[4:20])
    off = 20
    meta_sig = 0
    if version >= 2:
        (meta_sig,) = struct.unpack(">I", hdr[20:24])
        off = 24
    data_offset = off + manifest_size + meta_sig
    print(f"CrAU ver={version} manifest={manifest_size} meta_sig={meta_sig} data_off={data_offset}")

    # stream-count needles in payload
    print("payload needle counts:")
    for n, c in count_needles_file(payload_path).items():
        print(f"  {n!r}: {c}")

    xz_magic = b"\xfd7zXZ\x00"
    size = payload_path.stat().st_size
    # mmap-like scan via chunked find
    chunk = 16 * 1024 * 1024
    overlap = len(xz_magic) - 1
    prev = b""
    positions = []
    pos_base = 0
    with payload_path.open("rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            window = prev + b
            start = 0
            while True:
                j = window.find(xz_magic, start)
                if j < 0:
                    break
                abs_pos = pos_base - len(prev) + j
                if abs_pos >= data_offset:
                    positions.append(abs_pos)
                start = j + 1
            prev = window[-overlap:]
            pos_base += len(b)
    print(f"xz candidates={len(positions)} (showing first 30)")
    found = None
    for i, abs_pos in enumerate(positions[:30]):
        # try decompress with growing max
        with payload_path.open("rb") as f:
            f.seek(abs_pos)
            blob = f.read(min(120_000_000, size - abs_pos))
        for end in (2_000_000, 8_000_000, 32_000_000, len(blob)):
            try:
                dec = lzma.decompress(blob[:end])
            except Exception:
                continue
            if b"g5300q-" in dec or b"g5300g-" in dec or b"SupportedRatMap" in dec:
                print(f"HIT xz@{abs_pos} dec={len(dec)}")
                scan_bytes(dec, f"xz@{abs_pos}")
                found = dec
                break
        if found is not None:
            break
        if i % 5 == 4:
            print(f"  scanned {i+1}/{min(30,len(positions))}")
    return found


def main():
    print(f"zip size={ZIP.stat().st_size}")
    print("sha256...", flush=True)
    h = sha256_file(ZIP)
    print(f"sha256={h} ok={h == EXPECT_SHA}")
    print("whole-zip needles:")
    for n, c in count_needles_file(ZIP).items():
        print(f"  {n!r}: {c}")

    payload = extract_payload_member(ZIP)
    if payload is None:
        print("DONE (radio member path or no payload)")
        return 0

    carved = hunt_xz_in_payload(payload)
    if carved is not None:
        r = scan_bytes(carved, "carved")
        if r["interesting"]:
            out = OUT_DIR / f"STAGED-verizon-td1a221105003-{r['sha'][:12]}.img"
            out.write_bytes(carved)
            print("STAGED", out)
        else:
            # still save carved for inspection if g5300*
            if r["g5300q"] or r["g5300g"]:
                out = OUT_DIR / f"EXTRACT-verizon-td1a221105003-{r['sha'][:12]}.img"
                out.write_bytes(carved)
                print("EXTRACTED (not staged)", out)
    else:
        print("no modem-like xz carve")

    # cleanup large payload extract to save disk (optional keep for AP1A reuse pattern)
    try:
        payload.unlink()
        print("removed tmp payload")
    except Exception as e:
        print("cleanup", e)
    print("DONE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
