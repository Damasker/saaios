#!/usr/bin/env python3
from pathlib import Path
import hashlib

fw = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw")
for p in sorted(list(fw.glob("*.img")) + list((fw / "cdma-hunt").glob("*.img"))):
    data = p.read_bytes()
    h = hashlib.sha256(data).hexdigest()
    no = data.count(b"No CDMA in SupportedRatMap")
    en = data.count(b"EnableCdmaRat")
    gq = data.count(b"g5300q-")
    gg = data.count(b"g5300g-")
    i = data.find(b"g5300q-") if gq else data.find(b"g5300g-")
    ver = data[i : i + 56].split(b"\0", 1)[0] if i >= 0 else b""
    print(
        f"{p.name}: sha={h[:16]} size={len(data)} "
        f"g5300q={gq} g5300g={gg} no_cdma={no} enable={en} ver={ver!r}"
    )
