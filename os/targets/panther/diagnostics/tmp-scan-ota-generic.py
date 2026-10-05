#!/usr/bin/env python3
"""Generic stream-scan for a panther OTA zip: SHA, chip, CDMA RatMap."""
from __future__ import annotations

import hashlib
import lzma
import struct
import sys
import zipfile
from pathlib import Path

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
        "ver": ver,
        "interesting": gq > 0 and no == 0,
        "data": data,
    }


def hunt_xz(payload_path: Path, max_hits: int = 80) -> bytes | None:
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
    print(f"CrAU ver={version} manifest={manifest_size} data_off={data_offset}")
    print("payload needles:")
    for n, c in count_needles_file(payload_path).items():
        print(f"  {n!r}: {c}")

    xz_magic = b"\xfd7zXZ\x00"
    size = payload_path.stat().st_size
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
    print(f"xz candidates={len(positions)}")

    best = None
    best_score = -1
    scanned = 0
    for abs_pos in positions:
        if scanned >= max_hits and best is not None:
            break
        with payload_path.open("rb") as f:
            f.seek(abs_pos)
            blob = f.read(min(130_000_000, size - abs_pos))
        dec = None
        for end in (2_000_000, 8_000_000, 32_000_000, 120_000_000, len(blob)):
            try:
                dec = lzma.decompress(blob[:end])
            except Exception:
                continue
            break
        scanned += 1
        if not dec:
            continue
        gq = dec.count(b"g5300q-")
        gg = dec.count(b"g5300g-")
        rat = dec.count(b"SupportedRatMap")
        if not (gq or gg or rat):
            continue
        score = len(dec) + 10_000_000 * (gq + gg) + 1_000_000 * rat
        print(f"  cand xz@{abs_pos} size={len(dec)} gq={gq} gg={gg} rat={rat}")
        if score > best_score:
            best_score = score
            best = dec
    return best


def main():
    if len(sys.argv) < 2:
        print("usage: tmp-scan-ota-generic.py <zip> [expect_sha_prefix_or_full]")
        return 2
    zpath = Path(sys.argv[1])
    expect = sys.argv[2] if len(sys.argv) > 2 else ""
    tag = zpath.stem
    out_dir = zpath.parent
    print(f"zip={zpath} size={zpath.stat().st_size}")
    h = sha256_file(zpath)
    ok = (h == expect) if len(expect) == 64 else (h.startswith(expect) if expect else True)
    print(f"sha256={h} ok={ok}")
    print("whole-zip needles:")
    for n, c in count_needles_file(zpath).items():
        print(f"  {n!r}: {c}")

    payload_path = out_dir / f"tmp-payload-{tag}.bin"
    with zipfile.ZipFile(zpath, "r") as z:
        names = z.namelist()
        print(f"members={len(names)} {names[:20]}")
        for n in names:
            low = n.lower()
            if low.endswith(("radio.img", "modem.img")):
                data = z.read(n)
                r = scan_bytes(data, n)
                if r["interesting"]:
                    staged = out_dir / f"STAGED-{tag}-{r['sha'][:12]}.img"
                    staged.write_bytes(data)
                    print("STAGED", staged)
                else:
                    ext = out_dir / f"EXTRACT-{tag}-{r['sha'][:12]}.img"
                    ext.write_bytes(data)
                    print("EXTRACTED", ext)
                return 0
        if "payload.bin" not in names:
            print("no payload")
            return 1
        print("extracting payload.bin...")
        with z.open("payload.bin") as src, payload_path.open("wb") as dst:
            while True:
                c = src.read(8 * 1024 * 1024)
                if not c:
                    break
                dst.write(c)

    carved = hunt_xz(payload_path)
    if carved is not None:
        r = scan_bytes(carved, "carved-best")
        name = f"{'STAGED' if r['interesting'] else 'EXTRACT'}-{tag}-{r['sha'][:12]}.img"
        out = out_dir / name
        out.write_bytes(carved)
        print(("STAGED" if r["interesting"] else "EXTRACTED"), out)
        verdict = (
            "USABLE"
            if r["interesting"]
            else (
                "WRONG_CHIP_g5300g"
                if r["g5300g"] and not r["g5300q"]
                else (
                    "EU_NO_CDMA"
                    if r["g5300q"] and r["no_cdma"]
                    else "NO_MATCH"
                )
            )
        )
        print(f"VERDICT {verdict}")
    else:
        print("VERDICT NO_MODEM_CARVE")

    try:
        payload_path.unlink()
    except Exception as e:
        print("cleanup", e)
    print("DONE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
