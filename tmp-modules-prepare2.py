#!/usr/bin/env python3
import os, subprocess
from pathlib import Path

root = Path.home() / "local/debroot"
env = os.environ.copy()
env["PATH"] = f"{root/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{root/'usr/lib/x86_64-linux-gnu'}:{root/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["BISON_PKGDATADIR"] = str(root / "usr/share/bison")
env["M4"] = str(root / "usr/bin/m4")
host_inc = f"-I{root/'usr/include'} -I{root/'usr/include/x86_64-linux-gnu'}"

k = Path("/home/mike/kernel-work/common-bd23337")
cmd = [
    "make", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
    f"HOSTCFLAGS={host_inc}",
    f"-j{os.cpu_count() or 2}",
    "modules_prepare",
]
print("RUN", cmd, flush=True)
r = subprocess.run(cmd, cwd=k, env=env)
print("EXIT", r.returncode, flush=True)
rel = k / "include/config/kernel.release"
print("release", rel.read_text().strip() if rel.exists() else None)
sym = k / "Module.symvers"
print("symvers", sym.exists(), sym.stat().st_size if sym.exists() else 0)
raise SystemExit(r.returncode)
