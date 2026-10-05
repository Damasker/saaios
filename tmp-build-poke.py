#!/usr/bin/env python3
import os, shutil, subprocess
from pathlib import Path

root = Path.home() / "local/debroot"
env = os.environ.copy()
env["PATH"] = f"{root/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{root/'usr/lib/x86_64-linux-gnu'}:{root/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["BISON_PKGDATADIR"] = str(root / "usr/share/bison")
env["M4"] = str(root / "usr/bin/m4")
host_inc = f"-I{root/'usr/include'} -I{root/'usr/include/x86_64-linux-gnu'}"
host_ld = f"-L{root/'usr/lib/x86_64-linux-gnu'}"

k = Path("/home/mike/kernel-work/common-bd23337")
src = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
build = Path("/home/mike/kernel-work/saaios_cp_poke_build")
build.mkdir(parents=True, exist_ok=True)
shutil.copy2(src / "saaios_cp_poke.c", build / "saaios_cp_poke.c")
(build / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
# empty symvers is ok for basic APIs
(k / "Module.symvers").write_text("")

rel = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
cmd = [
    "make", f"-C{k}", f"M={build}",
    "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
    f"KERNELRELEASE={rel}",
    f"HOSTCFLAGS={host_inc}",
    f"HOSTLDFLAGS={host_ld}",
    "modules",
]
print("RUN", cmd, flush=True)
r = subprocess.run(cmd, cwd=build, env=env)
print("EXIT", r.returncode, flush=True)
ko = build / "saaios_cp_poke.ko"
print("ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
if ko.exists():
    out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
    for line in out.splitlines():
        if "vermagic=" in line:
            print(line)
            break
raise SystemExit(r.returncode)
