#!/usr/bin/env python3
"""Build saaios_shm_ro.ko with same clang/BTF recipe as poke."""
import os, re, shutil, subprocess
from pathlib import Path

CLANG = Path("/home/mike/local/clang/r487747c/bin")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
BUILD = Path("/home/mike/kernel-work/saaios_shm_ro_build")
KOS_MATCH = Path("/home/mike/kernel-work/phone-kos-match")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
STOCK = KOS_MATCH / "rfkill.ko"


def env_for_make():
    e = os.environ.copy()
    e["PATH"] = f"{CLANG}:{DEB/'usr/bin'}:{e.get('PATH','')}"
    e["LD_LIBRARY_PATH"] = (
        f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:"
        f"{e.get('LD_LIBRARY_PATH','')}"
    )
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    e["PAHOLE"] = "true"
    return e


def force_btf():
    ah = K / "include/generated/autoconf.h"
    text = ah.read_text()
    for name in ("CONFIG_DEBUG_INFO_BTF", "CONFIG_DEBUG_INFO_BTF_MODULES"):
        text = re.sub(rf"(?m)^#define {name} .*\n", "", text)
        text = re.sub(rf"(?m)^/\* {name} is not set \*/\n", "", text)
        text += f"#define {name} 1\n"
    ah.write_text(text)
    ac = K / "include/config/auto.conf"
    if ac.exists():
        t = ac.read_text()
        for key in ("CONFIG_DEBUG_INFO_BTF=y", "CONFIG_DEBUG_INFO_BTF_MODULES=y"):
            name = key.split("=")[0]
            t = re.sub(rf"(?m)^{name}=.*\n", "", t)
            t += key + "\n"
        ac.write_text(t)


def this_module_size(ko: Path) -> int:
    out = subprocess.check_output(
        [str(CLANG / "llvm-objdump"), "-h", str(ko)], text=True, errors="ignore"
    )
    for ln in out.splitlines():
        if ".gnu.linkonce.this_module" in ln and "rela" not in ln:
            m = re.search(r"this_module\s+([0-9a-fA-F]+)", ln)
            if m:
                return int(m.group(1), 16)
    raise RuntimeError("no this_module")


def main():
    force_btf()
    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_shm_ro.c", BUILD / "saaios_shm_ro.c")
    (BUILD / "Makefile").write_text(
        "obj-m += saaios_shm_ro.o\n"
        "CFLAGS_saaios_shm_ro.o += -fno-sanitize=kernel-address\n"
        "CFLAGS_saaios_shm_ro.mod.o += -fno-sanitize=kernel-address\n"
    )
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    cc, ld = str(CLANG / "clang"), str(CLANG / "ld.lld")
    base = [
        "make", f"-C{K}", "ARCH=arm64", f"CC={cc}", f"LD={ld}",
        "LLVM=1", "LLVM_IAS=1", "CLANG_TRIPLE=aarch64-linux-gnu-",
        "CROSS_COMPILE=aarch64-linux-gnu-", f"KERNELRELEASE={REL}",
        f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", "PAHOLE=true",
    ]
    r = subprocess.run(base + [f"M={BUILD}", "modules"], cwd=BUILD, env=env_for_make())
    ko = BUILD / "saaios_shm_ro.ko"
    print("EXIT", r.returncode, ko.exists())
    if not ko.exists():
        return r.returncode or 1
    ours = this_module_size(ko)
    stock = this_module_size(STOCK)
    print(f"this_module ours=0x{ours:x} stock=0x{stock:x}")
    shutil.copy2(ko, SRC / "saaios_shm_ro.ko")
    return 0 if ours == stock else 2


if __name__ == "__main__":
    raise SystemExit(main())
