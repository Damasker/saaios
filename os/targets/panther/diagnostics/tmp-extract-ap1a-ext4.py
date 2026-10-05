#!/usr/bin/env python3
"""Extract files from AP1A modem.img (ext4) and scan inner modem bins."""
from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

IMG = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/STAGED-ap1a-verizon-aa0dfc6160fc.img"
)
OUT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/ap1a-modem-fs"
)
EU = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/factory-cp2a.260705.006-modem.img"
)

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"SupportedRatMap",
    b"TCS_CDMA_SUPPORT",
    b"g5300q-",
    b"g5300g-",
]


def sha(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        while True:
            b = f.read(8 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def scan(data: bytes, label: str):
    print(f"SCAN {label} size={len(data)}")
    for n in NEEDLES:
        c = data.count(n)
        if c:
            print(f"  {n!r}: {c}")
    i = data.find(b"g5300q-")
    if i < 0:
        i = data.find(b"g5300g-")
    if i >= 0:
        print(f"  ver={data[i:i+56].split(bchr(0),1)[0] if False else data[i:i+56].split(b'\\0',1)}")
        print(f"  ver={data[i:i+56].split(bytes([0]),1)[0]!r}")
    print(f"  CDMA={data.count(b'CDMA')} head4={data[:4]!r}")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    print("img head", IMG.read_bytes()[:64])
    # Try debugfs ls
    tools = {t: shutil.which(t) for t in ("debugfs", "dumpe2fs", "7z", "unsquashfs")}
    print("tools", tools)

    mounted = False
    mnt = Path("/tmp/ap1a-modem-mnt")
    if os.geteuid() == 0 or True:
        mnt.mkdir(exist_ok=True)
        # try user-space fuse ext4 or loop mount
        r = subprocess.run(
            ["bash", "-lc", f"sudo -n mount -o loop,ro '{IMG}' '{mnt}'"],
            capture_output=True,
            text=True,
        )
        print("mount rc", r.returncode, r.stderr[:200] if r.stderr else "")
        if r.returncode == 0:
            mounted = True

    if not mounted and tools.get("debugfs"):
        r = subprocess.run(
            ["debugfs", "-R", "ls -l", str(IMG)], capture_output=True, text=True
        )
        print("debugfs ls:\n", r.stdout[:2000], r.stderr[:500])
        # try to extract modem.bin / *.bin
        for name in (
            "modem.bin",
            "modem_1.bin",
            "MAIN",
            "main.bin",
            "TOC",
            "image/modem.bin",
        ):
            dest = OUT / Path(name).name
            rr = subprocess.run(
                ["debugfs", "-R", f"dump /{name} {dest}", str(IMG)],
                capture_output=True,
                text=True,
            )
            print(f"dump {name} rc={rr.returncode} err={rr.stderr[:120]}")

    if mounted:
        for root, dirs, files in os.walk(mnt):
            rel = Path(root).relative_to(mnt)
            print("DIR", rel, "files", files[:30])
            for fn in files:
                src = Path(root) / fn
                dst = OUT / rel / fn
                dst.parent.mkdir(parents=True, exist_ok=True)
                if src.stat().st_size < 200_000_000:
                    shutil.copy2(src, dst)
                    print(f"copied {src} -> {dst} ({src.stat().st_size})")
        subprocess.run(["sudo", "-n", "umount", str(mnt)], check=False)

    # Fallback: carve ELF/TOC from ext4 by searching Shannon markers
    data = IMG.read_bytes()
    # Shannon modem TOC often has ASCII 'TOC' table; also look for nested version + large blob
    # If this is sparse ext4 with modem files, filenames appear as:
    for name in (b"modem.bin", b"modem_1.bin", b"nv_data.bin", b"TOC.bin", b"main.bin"):
        print(f"filename {name!r} count={data.count(name)}")

    # Compare with EU factory modem header
    if EU.exists():
        eu = EU.read_bytes()
        print(f"\nEU factory modem size={len(eu)} head={eu[:32]!r} sha={sha(EU)[:16]}")
        scan(eu, "EU-factory")
    scan(data, "AP1A-ext4-wrapper")

    # List extracted
    print("\nExtracted under", OUT)
    for p in sorted(OUT.rglob("*")):
        if p.is_file():
            b = p.read_bytes()
            print(f"FILE {p.relative_to(OUT)} size={len(b)} sha={sha(p)[:16]}")
            scan(b, str(p.name))
            interesting = b.count(b"g5300q-") > 0 and b.count(b"No CDMA in SupportedRatMap") == 0
            if interesting and len(b) > 30_000_000:
                staged = IMG.parent / f"STAGED-ap1a-inner-{sha(p)[:12]}.bin"
                staged.write_bytes(b)
                print("STAGED inner", staged)


if __name__ == "__main__":
    main()
