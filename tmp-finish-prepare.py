#!/usr/bin/env python3
import os, subprocess
from pathlib import Path

root = Path.home() / "local/debroot"
env = os.environ.copy()
env["PATH"] = f"{root/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{root/'usr/lib/x86_64-linux-gnu'}:{root/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["BISON_PKGDATADIR"] = str(root / "usr/share/bison")
env["M4"] = str(root / "usr/bin/m4")

k = Path("/home/mike/kernel-work/common-bd23337")
subprocess.check_call(["./scripts/config", "--disable", "TRIM_UNUSED_KSYMS"], cwd=k, env=env)
subprocess.check_call(["./scripts/config", "--set-str", "UNUSED_KSYMS_WHITELIST", ""], cwd=k, env=env)
subprocess.check_call(["make", "ARCH=arm64", "olddefconfig"], cwd=k, env=env)
print("OLDDEF_OK")
subprocess.check_call(
    ["make", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-", f"-j{os.cpu_count() or 2}", "modules_prepare"],
    cwd=k,
    env=env,
)
print("PREPARE_OK")
print((k / "include/config/kernel.release").read_text().strip())
sym = k / "Module.symvers"
print("symvers", sym.exists(), sym.stat().st_size if sym.exists() else 0)
