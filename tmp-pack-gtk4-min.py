#!/usr/bin/env python3
import os
import shutil
import subprocess
from pathlib import Path

sysroot = Path("/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot")
out = Path("/tmp/gtk4-probe")
if out.exists():
    shutil.rmtree(out)
(out / "bin").mkdir(parents=True)
(out / "lib").mkdir()
src_demo = sysroot / "usr/bin/gtk4-demo"
shutil.copy2(src_demo, out / "bin/gtk4-demo")
os.chmod(out / "bin/gtk4-demo", 0o755)

search = [sysroot / "lib", sysroot / "usr/lib"]


def needed(path: Path) -> list[str]:
    result = subprocess.run(
        ["readelf", "-d", str(path)], capture_output=True, text=True, check=False
    )
    names = []
    for line in result.stdout.splitlines():
        if "NEEDED" not in line:
            continue
        name = line.split("[", 1)[-1].split("]", 1)[0]
        if name.startswith("ld-") or name.startswith("libc.musl"):
            continue
        names.append(name)
    return names


def find_lib(name: str) -> Path | None:
    for root in search:
        direct = root / name
        if direct.exists():
            return direct
        for child in root.glob(name):
            return child
        for child in root.glob("*"):
            if child.is_dir():
                nested = child / name
                if nested.exists():
                    return nested
    return None


queue = [out / "bin/gtk4-demo"]
seen: set[str] = set()
while queue:
    path = queue.pop()
    for name in needed(path):
        if name in seen:
            continue
        found = find_lib(name)
        if found is None:
            print("missing", name)
            continue
        seen.add(name)
        dest = out / "lib" / name
        shutil.copy2(found, dest)
        queue.append(dest)

pixbuf = sysroot / "usr/lib/gdk-pixbuf-2.0"
if pixbuf.is_dir():
    dest = out / "lib/gdk-pixbuf-2.0"
    shutil.copytree(pixbuf, dest)

print("libs", len(seen))
subprocess.run(["du", "-sh", str(out)], check=False)
subprocess.run(
    ["qemu-aarch64-static", "-L", str(sysroot), str(out / "bin/gtk4-demo"), "--version"],
    check=False,
)
subprocess.run(["tar", "-C", "/tmp", "-cf", "/tmp/gtk4-probe.tar", "gtk4-probe"], check=True)
subprocess.run(["ls", "-lh", "/tmp/gtk4-probe.tar"], check=False)
