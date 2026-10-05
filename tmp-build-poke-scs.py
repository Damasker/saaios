#!/usr/bin/env python3
"""Re-enable SCS (+CFI if present in phone config), keep KASAN off, prepare, build."""
import os, re, shutil, struct, subprocess
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects/saaios-som")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = ROOT / "os/targets/panther/diagnostics"
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS = Path("/home/mike/kernel-work/phone-kos")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"
PHONE_CFG = ROOT / "os/targets/panther/diagnostics/phone-config.gz"


def env_for_make():
    e = os.environ.copy()
    e["PATH"] = f"{DEB/'usr/bin'}:{e.get('PATH','')}"
    e["LD_LIBRARY_PATH"] = (
        f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:"
        f"{e.get('LD_LIBRARY_PATH','')}"
    )
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    return e


def parse_ko(path: Path) -> dict[str, int]:
    raw = Path("/tmp") / (path.name + ".ver")
    subprocess.run(
        [OBJCOPY, "-O", "binary", "-j", "__versions", str(path), str(raw)],
        check=True, capture_output=True,
    )
    data = raw.read_bytes()
    syms = {}
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
    return syms


def fix_config():
    # Restore phone config then tweak
    import gzip
    phone = gzip.open(PHONE_CFG, "rt").read()
    (K / ".config").write_text(phone)
    # Apply LOCALVERSION and disable only KASAN (keep SCS/CFI)
    subprocess.run(
        ["scripts/config", "--disable", "LOCALVERSION_AUTO",
         "--set-str", "LOCALVERSION", "-android14-11-gbd23337e42e7-ab14791245",
         "--disable", "KASAN", "--disable", "KASAN_GENERIC",
         "--disable", "KASAN_INLINE", "--disable", "KASAN_OUTLINE",
         "--disable", "TRIM_UNUSED_KSYMS",
         "--enable", "SHADOW_CALL_STACK",
         "--enable", "CFI_CLANG"],
        cwd=K, check=False,
    )
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    env = env_for_make()
    for target in ("olddefconfig", "modules_prepare"):
        cmd = [
            "make", f"-C{K}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
            f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}",
            target,
        ]
        print("RUN", target, flush=True)
        r = subprocess.run(cmd, env=env)
        if r.returncode != 0:
            return r.returncode
    cfg = (K / ".config").read_text()
    for k in ("CONFIG_SHADOW_CALL_STACK", "CONFIG_CFI_CLANG", "CONFIG_KASAN", "CONFIG_MODVERSIONS"):
        print([ln for ln in cfg.splitlines() if ln.startswith(k) or ln.startswith(f"# {k}")][:3])
    return 0


def main():
    rc = fix_config()
    if rc:
        return rc
    syms = {}
    for ko in sorted(KOS.glob("*.ko")):
        try:
            syms.update(parse_ko(ko))
        except Exception as ex:
            print("fail", ko.name, ex)
    lines = [f"0x{crc:08x}\t{name}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for name, crc in sorted(syms.items())]
    (K / "Module.symvers").write_text("".join(lines))
    print("symvers", len(lines), "printk", hex(syms.get("_printk", 0)))

    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    cmd = [
        "make", f"-C{K}", f"M={BUILD}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", "modules",
    ]
    r = subprocess.run(cmd, cwd=BUILD, env=env_for_make())
    ko = BUILD / "saaios_cp_poke.ko"
    print("EXIT", r.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
    if ko.exists():
        shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
        out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
        for line in out.splitlines():
            if "vermagic=" in line:
                print(line)
                break
    return r.returncode


if __name__ == "__main__":
    raise SystemExit(main())
