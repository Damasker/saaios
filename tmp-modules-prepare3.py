#!/usr/bin/env python3
import os, subprocess, re
from pathlib import Path

root = Path.home() / "local/debroot"
env = os.environ.copy()
env["PATH"] = f"{root/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{root/'usr/lib/x86_64-linux-gnu'}:{root/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["BISON_PKGDATADIR"] = str(root / "usr/share/bison")
env["M4"] = str(root / "usr/bin/m4")
host_inc = f"-I{root/'usr/include'} -I{root/'usr/include/x86_64-linux-gnu'}"
host_ld = f"-L{root/'usr/lib/x86_64-linux-gnu'} -L{root/'lib/x86_64-linux-gnu'}"

k = Path("/home/mike/kernel-work/common-bd23337")
# Disable BTF tooling (resolve_btfids fails on host gcc -Werror)
for opt in ["DEBUG_INFO_BTF", "DEBUG_INFO_BTF_MODULES", "MODULE_SIG", "MODULE_SIG_ALL", "MODULE_SIG_FORCE"]:
    subprocess.call(["./scripts/config", "--disable", opt], cwd=k, env=env)
subprocess.check_call(["make", "ARCH=arm64", "olddefconfig"], cwd=k, env=env)
print("OLDDEF_OK", flush=True)

cmd = [
    "make", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
    f"HOSTCFLAGS={host_inc}",
    f"HOSTLDFLAGS={host_ld}",
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
