#!/usr/bin/env python3
"""Targeted radio-APEX / non-vendor oem_ipc hunt (follow-up to tmp-extract-factory-nonvendor).

Scans system/product/system_ext/system_other for:
  com.android.hardware.radio*, SIM_INIT_REQ, IpcTxSimInit, /dev/oem_ipc, 0x2f50
No invent. Offline only.
"""
from __future__ import annotations

import struct
import zipfile
import io
from pathlib import Path

ROOT = Path(__file__).resolve().parent
IMGDIR = ROOT / "fw" / "cdma-hunt" / "factory-td1a-images"
OUT = ROOT / "tmp-nonvendor-apex-radio.out"
APEX_OUT = IMGDIR / "carved-apex"

NEEDLES = [
    b"com.android.hardware.radio",
    b"com.android.hardware.radio.deprecated",
    b"com.google.android.ril",
    b"hardware.radio",
    b"rild_exynos",
    b"libsitril",
    b"libsec-ril",
    b"SitOem",
    b"SIM_INIT_REQ",
    b"IpcTxSimInit",
    b"SimInitMessage",
    b"/dev/oem_ipc",
    b"oem_ipc0",
    b"0x2f50",
    b"oem_ipc_message",
]


def log(lines: list[str], s: str) -> None:
    print(s, flush=True)
    lines.append(s)


def scan_file(path: Path, lines: list[str]) -> dict[bytes, list[int]]:
    size = path.stat().st_size
    log(lines, f"\n## {path.name} size={size}")
    hits: dict[bytes, list[int]] = {n: [] for n in NEEDLES}
    if size <= 2_200_000_000:
        data = path.read_bytes()
        for n in NEEDLES:
            start = 0
            while len(hits[n]) < 40:
                i = data.find(n, start)
                if i < 0:
                    break
                hits[n].append(i)
                start = i + 1
        # keep data for apex carve below
        path_data = data
    else:
        path_data = None
        with path.open("rb") as f:
            prev = b""
            off = 0
            ov = 256
            while True:
                chunk = f.read(32 << 20)
                if not chunk:
                    break
                buf = prev + chunk
                for n in NEEDLES:
                    start = 0
                    while len(hits[n]) < 40:
                        i = buf.find(n, start)
                        if i < 0:
                            break
                        abs_off = off - len(prev) + i
                        if abs_off >= 0 and abs_off not in hits[n]:
                            hits[n].append(abs_off)
                        start = i + 1
                prev = buf[-ov:]
                off += len(chunk)

    for n in NEEDLES:
        offs = hits[n]
        log(lines, f"  {n!r}: {len(offs)} first={offs[:12]}")
        if n.startswith(b"com.android.hardware.radio") or n in (
            b"hardware.radio",
            b"/dev/oem_ipc",
            b"SIM_INIT_REQ",
            b"IpcTxSimInit",
        ):
            if path_data is not None:
                for o in offs[:6]:
                    ctx = path_data[max(0, o - 24) : o + 80]
                    s = "".join(chr(c) if 32 <= c < 127 else "." for c in ctx)
                    log(lines, f"    @{o:#x}: {s}")
    return hits, path_data


def try_extract_apex_named(data: bytes, img_name: str, lines: list[str]) -> list[Path]:
    """Report apex name strings; carve only ZIPs whose apex_manifest mentions radio."""
    APEX_OUT.mkdir(parents=True, exist_ok=True)
    carved: list[Path] = []
    keys = (
        b"com.android.hardware.radio",
        b"com.android.hardware.radio.deprecated",
    )
    name_hits = []
    for key in keys:
        start = 0
        while len(name_hits) < 30:
            i = data.find(key, start)
            if i < 0:
                break
            end = data.find(b"\x00", i)
            if end < 0 or end - i > 200:
                end = i + min(120, len(data) - i)
            name_hits.append((i, data[i:end]))
            start = i + 1
    log(lines, f"\n### apex name strings in {img_name}: {len(name_hits)}")
    for off, raw in name_hits[:20]:
        log(lines, f"  @{off:#x}: {raw.decode('ascii', 'replace')}")

    start = 0
    apex_path_hits = []
    while len(apex_path_hits) < 40:
        i = data.find(b".apex", start)
        if i < 0:
            break
        j = i
        while j > 0 and 32 <= data[j - 1] < 127 and data[j - 1] not in b"\n\r\t ":
            j -= 1
            if i - j > 180:
                break
        path = data[j : i + 5]
        if b"radio" in path.lower() or b"ril" in path.lower():
            apex_path_hits.append((j, path))
        start = i + 5
    log(lines, f"  .apex paths with radio/ril: {len(apex_path_hits)}")
    for off, raw in apex_path_hits[:20]:
        log(lines, f"    @{off:#x}: {raw.decode('ascii', 'replace')}")

    # Fast carve: only try PK headers within ±2MiB of a radio apex name hit.
    seed_offs = [o for o, _ in name_hits] + [o for o, _ in apex_path_hits]
    tried = set()
    for seed in seed_offs:
        lo = max(0, seed - (2 << 20))
        hi = min(len(data), seed + (2 << 20))
        region = data[lo:hi]
        pos = 0
        while True:
            i = region.find(b"PK\x03\x04", pos)
            if i < 0:
                break
            abs_i = lo + i
            if abs_i in tried:
                pos = i + 4
                continue
            tried.add(abs_i)
            window = data[abs_i : abs_i + min(len(data) - abs_i, 24 << 20)]
            eocd = window.rfind(b"PK\x05\x06")
            if eocd < 0 or eocd > (16 << 20):
                pos = i + 4
                continue
            blob = window[: eocd + 22]
            try:
                zf = zipfile.ZipFile(io.BytesIO(blob))
            except zipfile.BadZipFile:
                pos = i + 4
                continue
            names = zf.namelist()
            joined = " ".join(names).lower()
            is_apex = "apex_payload" in joined or "apex_manifest" in joined
            manifest_hit = False
            for n in names:
                if "apex_manifest" not in n.lower():
                    continue
                try:
                    m = zf.read(n)
                except Exception:
                    continue
                if b"radio" in m.lower():
                    manifest_hit = True
                    break
            if is_apex and manifest_hit:
                dest = APEX_OUT / f"{img_name}-apex-{abs_i:x}.zip"
                dest.write_bytes(blob)
                log(
                    lines,
                    f"  CARVED radio APEX @{abs_i:#x} -> {dest.name} entries={len(names)}",
                )
                carved.append(dest)
                for n in names:
                    if "apex_payload" not in n.lower() and not n.endswith(".so"):
                        continue
                    try:
                        payload = zf.read(n)
                    except Exception as e:
                        log(lines, f"    read {n} fail: {e}")
                        continue
                    pdest = APEX_OUT / f"{img_name}-apex-{abs_i:x}-{Path(n).name}"
                    pdest.write_bytes(payload)
                    log(lines, f"    wrote {pdest.name} size={len(payload)}")
                    for nn in (
                        b"SIM_INIT_REQ",
                        b"IpcTxSimInit",
                        b"/dev/oem_ipc",
                        b"oem_ipc0",
                        b"0x2f50",
                    ):
                        c = payload.count(nn)
                        if c:
                            log(lines, f"      HIT {nn!r} count={c}")
            pos = i + 4
    if not carved:
        log(lines, "  no radio APEX ZIP carved near name hits (may be erofs/ext4-packed)")
    return carved


def main() -> None:
    lines: list[str] = ["tmp-nonvendor-apex-radio.py"]
    magic = (IMGDIR / "system.img").read_bytes()[:16]
    log(lines, f"system.img head={magic.hex()} sparse={magic[:4]==bytes.fromhex('3aff26ed')}")

    all_hits = {}
    for name in ("system.img", "product.img", "system_ext.img", "system_other.img"):
        p = IMGDIR / name
        if not p.exists():
            log(lines, f"MISSING {name}")
            continue
        hits, data = scan_file(p, lines)
        all_hits[name] = hits
        if data is not None:
            try_extract_apex_named(data, name, lines)

    # Summary
    log(lines, "\n## SUMMARY")
    encoder = False
    for img, hits in all_hits.items():
        sim = hits.get(b"SIM_INIT_REQ", [])
        ipc = hits.get(b"IpcTxSimInit", [])
        oem = hits.get(b"/dev/oem_ipc", []) + hits.get(b"oem_ipc0", [])
        radio = hits.get(b"com.android.hardware.radio", [])
        log(
            lines,
            f"  {img}: SIM_INIT_REQ={len(sim)} IpcTxSimInit={len(ipc)} "
            f"oem_ipc={len(oem)} hardware.radio={len(radio)}",
        )
        if sim or ipc or (oem and hits.get(b"0x2f50")):
            encoder = True
    if not encoder:
        log(
            lines,
            "  Encoder found? NO — catalog 0x2f50 encoder not in "
            "system/product/system_ext/system_other (name-level + oem_ipc).",
        )
    else:
        log(lines, "  Encoder found? POSSIBLE — inspect hits")

    # Note: AOSP radio APEX is not expected to hold Shannon catalog OEM encode;
    # Shannon host path remains vendor cbd/rild/SitOem (already closed).
    log(
        lines,
        "  Note: Shannon catalog OEM SIM_INIT lives in CP catalog + host "
        "writer; AOSP com.android.hardware.radio APEX is IRadio HAL, not "
        "oem_ipc catalog encoder.",
    )
    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"WROTE {OUT}")


if __name__ == "__main__":
    main()
