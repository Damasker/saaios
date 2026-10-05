#!/usr/bin/env python3
"""Dump AP1A PIXELMODEM ext4 contents and scan for CDMA RatMap."""
from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

IMG = "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/STAGED-ap1a-verizon-aa0dfc6160fc.img"
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/ap1a-modem-fs"
)
EU = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-cp2a.260705.006-modem.img"
)
OUT.mkdir(parents=True, exist_ok=True)

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"SupportedRatMap",
    b"TCS_CDMA_SUPPORT",
    b"g5300q-",
    b"g5300g-",
]


def run_debugfs(commands: str) -> str:
    r = subprocess.run(
        ["debugfs", IMG], input=commands + "\nquit\n", capture_output=True, text=True
    )
    return r.stdout + "\n" + r.stderr


def sha(p: Path) -> str:
    h = hashlib.sha256()
    h.update(p.read_bytes())
    return h.hexdigest()


def scan(data: bytes, label: str):
    print(f"SCAN {label} size={len(data)} sha={hashlib.sha256(data).hexdigest()[:16]}")
    for n in NEEDLES:
        c = data.count(n)
        if c:
            print(f"  {n!r}: {c}")
    print(f"  CDMA={data.count(b'CDMA')} head={data[:16]!r}")


ver = "g5300q-231218-240405-B-11675365"
print("=== list version dir ===")
print(run_debugfs(f"cd images\ncd {ver}\nls -l\n"))

print("=== list SPI ===")
print(run_debugfs("cd images\ncd SPI\nls -l\n"))

print("=== readlink default ===")
print(run_debugfs("cd images\nstat default\n"))

# Export entire tree via debugfs rdump
print("=== rdump /images ===")
# clear out
for p in OUT.glob("**/*"):
    if p.is_file():
        p.unlink()
rd = OUT / "rdump"
rd.mkdir(exist_ok=True)
# debugfs rdump
out = run_debugfs(f"rdump /images {rd}")
print(out[:2000])

# Walk extracted
print("=== extracted files ===")
files = sorted(rd.rglob("*"))
for p in files:
    if p.is_file():
        print(f"FILE {p.relative_to(rd)} size={p.stat().st_size}")

# Scan interesting bins
candidates = []
for p in files:
    if not p.is_file():
        continue
    if p.stat().st_size < 1000:
        continue
    data = p.read_bytes()
    scan(data, str(p.relative_to(rd)))
    if b"g5300q-" in data or b"SupportedRatMap" in data or b"CDMA" in data:
        candidates.append(p)

print("\n=== EU compare (same packaging?) ===")
if EU.exists():
    # try debugfs on EU too
    r = subprocess.run(
        ["debugfs", str(EU)],
        input="stats\nls -l\nquit\n",
        capture_output=True,
        text=True,
    )
    print(r.stdout[:1500])
    eu = EU.read_bytes()
    scan(eu, "EU-whole")

print("\n=== UDL decision inputs ===")
# Prefer largest file with RatMap / MAIN
best = None
for p in files:
    if not p.is_file():
        continue
    data = p.read_bytes()
    if len(data) < 1_000_000:
        continue
    no = data.count(b"No CDMA in SupportedRatMap")
    gq = data.count(b"g5300q-")
    rat = data.count(b"SupportedRatMap")
    print(
        f"cand {p.relative_to(rd)} size={len(data)} gq={gq} no_cdma={no} rat={rat} CDMA={data.count(b'CDMA')}"
    )
    if gq and len(data) > 10_000_000:
        best = (p, data, no, gq, rat)

if best:
    p, data, no, gq, rat = best
    interesting = gq > 0 and no == 0 and len(data) > 30_000_000
    print(f"BEST {p} interesting={interesting}")
    dest_name = (
        f"STAGED-ap1a-inner-{sha(p)[:12]}.bin"
        if interesting
        else f"EXTRACT-ap1a-inner-{sha(p)[:12]}.bin"
    )
    dest = Path(IMG).parent / dest_name
    dest.write_bytes(data)
    print("WROTE", dest)
else:
    print("NO best inner modem candidate")
