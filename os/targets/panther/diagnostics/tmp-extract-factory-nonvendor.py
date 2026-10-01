#!/usr/bin/env python3
"""Extract + scan factory-td1a partitions beyond vendor for catalog 0x2f50 encoder.

Offline only. No invent. No live inject. No secrets.
Targets: system / product / system_ext / radio-related APEX.
Needles: oem_ipc, SIM_INIT_REQ, IpcTxSimInit, ASCII 0x2f50, MOVZ #0x2f50.
"""
from __future__ import annotations

import os
import shutil
import struct
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
HUNT = ROOT / "fw" / "cdma-hunt"
FACTORY_ZIP = HUNT / "panther-td1a.221105.001-factory-10a338fe.zip"
INNER_NAME = "panther-td1a.221105.001/image-panther-td1a.221105.001.zip"
OUT = HUNT / "factory-td1a-images"
INNER_ZIP = OUT / "image-panther-td1a.221105.001.zip"
LIST_TXT = OUT / "image-list.txt"
SCAN_OUT = ROOT / "tmp-nonvendor-2f50-hunt.out"

# Prefer these members from the image zip (names may vary).
# Skip boot/vendor_boot — radio APEX + telephony live in system/product/system_ext/super.
WANT_SUBSTR = (
    "system.img",
    "product.img",
    "system_ext.img",
    "system_other.img",
    "super.img",
    ".apex",
)

STRING_NEEDLES = (
    b"SIM_INIT_REQ",
    b"IpcTxSimInit",
    b"SimInitMessage",
    b"0x2f50",
    b"/dev/oem_ipc",
    b"oem_ipc0",
    b"oem_ipc_message",
    b"USIM_WAIT_FOR_INIT",
    b"SIM_VERIFYPIN_REQ",
)

ELF_MAGIC = b"\x7fELF"
APEX_MAGIC_HINTS = (b"APEX", b"com.android.hardware.radio", b"com.android.r")


def log(msg: str, fh) -> None:
    print(msg, flush=True)
    fh.write(msg + "\n")
    fh.flush()


def ensure_inner_zip(fh) -> Path:
    OUT.mkdir(parents=True, exist_ok=True)
    if INNER_ZIP.exists() and INNER_ZIP.stat().st_size > 1_000_000:
        log(f"inner zip present: {INNER_ZIP} ({INNER_ZIP.stat().st_size})", fh)
        return INNER_ZIP
    if not FACTORY_ZIP.exists():
        raise SystemExit(f"missing factory zip: {FACTORY_ZIP}")
    log(f"extracting inner image zip from {FACTORY_ZIP.name} ...", fh)
    with zipfile.ZipFile(FACTORY_ZIP) as z:
        with z.open(INNER_NAME) as src, INNER_ZIP.open("wb") as dst:
            shutil.copyfileobj(src, dst, 8 << 20)
    log(f"wrote {INNER_ZIP} ({INNER_ZIP.stat().st_size})", fh)
    return INNER_ZIP


def list_inner(fh) -> list[tuple[str, int]]:
    rows = []
    with zipfile.ZipFile(INNER_ZIP) as z:
        for name in sorted(z.namelist()):
            info = z.getinfo(name)
            rows.append((name, info.file_size))
            log(f"{info.file_size:12d}  {name}", fh)
    LIST_TXT.write_text("\n".join(f"{sz:12d}  {n}" for n, sz in rows) + "\n")
    return rows


def want_member(name: str) -> bool:
    base = name.split("/")[-1].lower()
    if base in ("vendor.img",):
        return False  # already hunted
    for s in WANT_SUBSTR:
        if s in base:
            return True
    # also grab apex payloads if named oddly
    if "radio" in base and (base.endswith(".apex") or base.endswith(".img")):
        return True
    return False


def extract_wanted(fh) -> list[Path]:
    extracted: list[Path] = []
    with zipfile.ZipFile(INNER_ZIP) as z:
        for name in z.namelist():
            if not want_member(name):
                continue
            base = Path(name).name
            dest = OUT / base
            info = z.getinfo(name)
            if dest.exists() and dest.stat().st_size == info.file_size:
                log(f"skip existing {dest.name} ({dest.stat().st_size})", fh)
                extracted.append(dest)
                continue
            log(f"extract {name} -> {dest} ({info.file_size})", fh)
            with z.open(name) as src, dest.open("wb") as dst:
                shutil.copyfileobj(src, dst, 8 << 20)
            extracted.append(dest)
    return extracted


def count_needle(data: bytes, needle: bytes, limit: int = 50) -> list[int]:
    hits = []
    start = 0
    while len(hits) < limit:
        i = data.find(needle, start)
        if i < 0:
            break
        hits.append(i)
        start = i + 1
    return hits


def movz_imm16_hits(data: bytes, imm: int = 0x2F50, limit: int = 200) -> list[int]:
    """AArch64 MOVZ Wd/Xd, #imm16, LSL#0."""
    hits = []
    for opc_base in (0x52800000, 0xD2800000):  # W, X
        for rd in range(32):
            w = struct.pack("<I", opc_base | ((imm & 0xFFFF) << 5) | rd)
            start = 0
            while len(hits) < limit:
                i = data.find(w, start)
                if i < 0:
                    break
                if (i % 4) == 0:
                    hits.append(i)
                start = i + 1
            if len(hits) >= limit:
                return sorted(set(hits))
    return sorted(set(hits))


def scan_blob(label: str, data: bytes, fh) -> dict:
    res = {
        "label": label,
        "size": len(data),
        "strings": {},
        "movz_2f50": [],
        "elf_count_hint": data.count(ELF_MAGIC),
    }
    log(f"\n=== SCAN {label} size={len(data)} ===", fh)
    for n in STRING_NEEDLES:
        offs = count_needle(data, n)
        res["strings"][n.decode("ascii", "replace")] = offs
        if offs:
            log(f"  {n!r}: count={len(offs)} first={offs[:8]}", fh)
        else:
            log(f"  {n!r}: 0", fh)
    movz = movz_imm16_hits(data)
    res["movz_2f50"] = movz
    log(f"  MOVZ #0x2f50 sites: {len(movz)} first={movz[:12]}", fh)
    log(f"  ELF magic count: {res['elf_count_hint']}", fh)

    # If oem_ipc and MOVZ co-located in same 256 KiB window → candidate
    oem_offs = res["strings"].get("/dev/oem_ipc", []) + res["strings"].get("oem_ipc0", [])
    if oem_offs and movz:
        for o in oem_offs[:20]:
            near = [m for m in movz if abs(m - o) < 256 * 1024]
            if near:
                log(f"  CANDIDATE window: oem@{o:#x} near movz={near[:5]}", fh)
    return res


def carve_apex_zips(data: bytes, parent: str, fh) -> list[tuple[str, bytes]]:
    """Find embedded ZIP (APEX) by PK\x03\x04 and extract members named .so/.bin."""
    out = []
    start = 0
    found = 0
    while found < 40:
        i = data.find(b"PK\x03\x04", start)
        if i < 0:
            break
        # Try open as zip from this offset via temp slice (bounded)
        # Heuristic: look ahead for EOCD within 64MB
        window = data[i : i + min(len(data) - i, 64 << 20)]
        eocd = window.rfind(b"PK\x05\x06")
        if eocd < 0:
            start = i + 4
            continue
        blob = window[: eocd + 22]
        try:
            import io

            zf = zipfile.ZipFile(io.BytesIO(blob))
        except zipfile.BadZipFile:
            start = i + 4
            continue
        names = zf.namelist()
        interesting = [
            n
            for n in names
            if any(
                k in n.lower()
                for k in ("radio", "ril", "sit", "oem", "ipc", "sim", "telephony")
            )
            or n.endswith((".so", ".bin", "apex_payload.img", "apex_payload"))
        ]
        radioish = any("radio" in n.lower() or "ril" in n.lower() for n in names)
        if radioish or interesting:
            log(
                f"  embedded ZIP @{i:#x} entries={len(names)} radioish={radioish}",
                fh,
            )
            for n in names:
                if "radio" in n.lower() or n.endswith(".so") or "apex_payload" in n:
                    try:
                        out.append((f"{parent}+zip@{i:#x}/{n}", zf.read(n)))
                    except Exception as e:
                        log(f"    read fail {n}: {e}", fh)
        found += 1
        start = i + 4
    return out


def scan_path(path: Path, fh) -> list[dict]:
    results = []
    # Stream in chunks for huge images but keep whole file if < 1.5GB for MOVZ
    size = path.stat().st_size
    log(f"\n## file {path.name} size={size}", fh)
    if size > 2_500_000_000:
        log("  too large for full RAM load; chunk-scan strings only", fh)
        # chunk string scan
        needle_hits = {n.decode(): [] for n in STRING_NEEDLES}
        movz_total = 0
        with path.open("rb") as f:
            overlap = 64
            prev = b""
            off = 0
            while True:
                chunk = f.read(32 << 20)
                if not chunk:
                    break
                data = prev + chunk
                for n in STRING_NEEDLES:
                    for h in count_needle(data, n, limit=20):
                        abs_off = off - len(prev) + h
                        if abs_off >= off - (len(prev) - overlap if prev else 0):
                            needle_hits[n.decode()].append(abs_off)
                movz_total += len(movz_imm16_hits(data, limit=50))
                prev = data[-overlap:]
                off += len(chunk)
        for k, v in needle_hits.items():
            log(f"  {k!r}: count={len(v)} first={v[:8]}", fh)
        log(f"  MOVZ #0x2f50 partial_count≈{movz_total}", fh)
        results.append({"label": path.name, "strings": needle_hits, "chunked": True})
        return results

    data = path.read_bytes()
    results.append(scan_blob(path.name, data, fh))

    # If looks like sparse/ext4 with embedded APEX zips, carve radio ones
    if path.suffix == ".img" and (
        b"com.android.hardware.radio" in data
        or b"com.android.rild" in data
        or b".apex" in data[: min(len(data), 8 << 20)]
        or b"apex" in data
    ):
        log(f"  trying embedded ZIP/APEX carve in {path.name}...", fh)
        for label, blob in carve_apex_zips(data, path.name, fh):
            if len(blob) < 64:
                continue
            # nested payload may itself be ext4/erofs image
            results.append(scan_blob(label, blob, fh))
            if blob[:4] == ELF_MAGIC or blob[:4] == b"PK\x03\x04":
                continue
            # second-level: if apex_payload.img, scan again for ELFs via ZIP already done;
            # also string-scan for needles already in scan_blob
    return results


def summarize(all_res: list[dict], fh) -> bool:
    """Return True if any encoder-like hit found."""
    log("\n## SUMMARY", fh)
    encoder = False
    for r in all_res:
        label = r.get("label", "?")
        strings = r.get("strings", {})
        sim = strings.get("SIM_INIT_REQ", [])
        ipc = strings.get("IpcTxSimInit", [])
        oem = strings.get("/dev/oem_ipc", []) + strings.get("oem_ipc0", [])
        movz = r.get("movz_2f50", [])
        if sim or ipc:
            log(f"  HIT name-level {label}: SIM_INIT_REQ={len(sim)} IpcTxSimInit={len(ipc)}", fh)
            encoder = True
        if oem and movz:
            log(f"  HIT oem_ipc+MOVZ {label}: oem={len(oem)} movz={len(movz)}", fh)
            encoder = True
        ascii_2f50 = strings.get("0x2f50", [])
        if ascii_2f50 and oem:
            log(f"  HIT ascii 0x2f50 + oem_ipc {label}", fh)
            encoder = True
    if not encoder:
        log("  Encoder found? NO (no SIM_INIT_REQ/IpcTxSimInit; no oem_ipc∧MOVZ#0x2f50)", fh)
    else:
        log("  Encoder found? POSSIBLE — inspect hits above before any send", fh)
    return encoder


def main() -> int:
    with SCAN_OUT.open("w", encoding="utf-8") as fh:
        log("tmp-extract-factory-nonvendor.py", fh)
        ensure_inner_zip(fh)
        log("\n## image zip members", fh)
        rows = list_inner(fh)
        log("\n## extracting wanted partitions", fh)
        paths = extract_wanted(fh)
        # Also scan any .apex already beside
        for p in sorted(OUT.glob("*.apex")):
            if p not in paths:
                paths.append(p)
        log(f"\n## scanning {len(paths)} files", fh)
        all_res: list[dict] = []
        for p in paths:
            try:
                all_res.extend(scan_path(p, fh))
            except Exception as e:
                log(f"ERROR scanning {p}: {e}", fh)
        summarize(all_res, fh)
        log(f"\nWrote {SCAN_OUT}", fh)
    print(f"OUT={SCAN_OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
