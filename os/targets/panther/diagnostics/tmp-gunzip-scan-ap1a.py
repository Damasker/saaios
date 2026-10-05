#!/usr/bin/env python3
from __future__ import annotations

import gzip
import hashlib
import subprocess
from pathlib import Path

HUNT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt"
)
GZ = (
    HUNT
    / "ap1a-modem-fs/rdump/images/g5300q-231218-240405-B-11675365/modem.bin.gz"
)
OUT = HUNT / "EXTRACT-ap1a-verizon-modem.bin"
EU_IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/"
    "factory-cp2a.260705.006-modem.img"
)
EU_OUT = HUNT / "EXTRACT-eu-cp2a-modem.bin"

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"SupportedRatMap",
    b"TCS_CDMA_SUPPORT",
    b"g5300q-",
    b"g5300g-",
]


def scan(data: bytes, label: str) -> None:
    h = hashlib.sha256(data).hexdigest()
    print(f"SCAN {label} size={len(data)} sha={h} head={data[:24]!r}")
    for n in NEEDLES:
        c = data.count(n)
        if c:
            print(f"  {n!r}: {c}")
    print(f"  CDMA={data.count(b'CDMA')}")
    i = data.find(b"g5300q-")
    if i >= 0:
        print(f"  ver={data[i:i+56].split(b'\\0'[:1],1)}")  # noqa placeholder
        print(f"  ver={data[i:i+56].split(bytes([0]),1)[0]!r}")


print("gunzip AP1A", GZ, "size", GZ.stat().st_size)
raw = gzip.decompress(GZ.read_bytes())
OUT.write_bytes(raw)
scan(raw, "AP1A-inner-modem.bin")

print("\nextract EU PIXELMODEM...")
rd = Path("/tmp/eu-modem-rdump")
rd.mkdir(exist_ok=True)
subprocess.run(
    ["debugfs", str(EU_IMG)],
    input=f"rdump /images {rd}\nquit\n",
    capture_output=True,
    text=True,
)
gzs = list(rd.rglob("modem.bin.gz"))
print("EU gz", gzs)
for g in gzs:
    if "SPI" in str(g):
        continue
    eu = gzip.decompress(g.read_bytes())
    EU_OUT.write_bytes(eu)
    scan(eu, f"EU-inner-from-{g.name}")
    break

no = raw.count(b"No CDMA in SupportedRatMap")
en = raw.count(b"EnableCdmaRat")
gq = raw.count(b"g5300q-")
interesting = gq > 0 and no == 0 and len(raw) > 30_000_000
print(
    f"\nVERDICT interesting={interesting} gq={gq} no_cdma={no} enable={en} size={len(raw)}"
)
if interesting:
    h = hashlib.sha256(raw).hexdigest()
    staged = HUNT / f"STAGED-ap1a-verizon-inner-{h[:12]}.bin"
    staged.write_bytes(raw)
    print("STAGED", staged)
else:
    print("NOT STAGING — no UDL")
