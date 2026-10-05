#!/usr/bin/env python3
"""Rebuild poke.ko using CRCs only from vermagic-matching phone modules."""
import os, re, shutil, struct, subprocess, urllib.request
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects/saaios-som")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = ROOT / "os/targets/panther/diagnostics"
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS = Path("/home/mike/kernel-work/phone-kos-match")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
MATCH = "gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"
PHONE = "http://172.31.7.1:8080"
FETCH = ["bluetooth.ko", "btbcm.ko", "btqca.ko", "hci_uart.ko", "rfkill.ko", "zsmalloc.ko",
         "cfg80211.ko", "bcmdhd4389.ko"]  # may be wrong vermagic; filtered


def env_for_make():
    e = os.environ.copy()
    e["PATH"] = f"{DEB/'usr/bin'}:{e.get('PATH','')}"
    e["LD_LIBRARY_PATH"] = f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:{e.get('LD_LIBRARY_PATH','')}"
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    return e


def vermagic_of(path: Path) -> str:
    out = subprocess.check_output(["strings", str(path)], text=True, errors="ignore")
    for line in out.splitlines():
        if line.startswith("vermagic="):
            return line
    return ""


def parse_ko(path: Path) -> dict[str, int]:
    raw = Path("/tmp") / (path.name + ".ver")
    subprocess.run([OBJCOPY, "-O", "binary", "-j", "__versions", str(path), str(raw)],
                   check=True, capture_output=True)
    data = raw.read_bytes()
    syms = {}
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
    return syms


def main():
    KOS.mkdir(parents=True, exist_ok=True)
    for name in FETCH:
        dest = KOS / name
        print("GET", name, flush=True)
        try:
            urllib.request.urlretrieve(f"{PHONE}/{name}", dest)
        except Exception as ex:
            print("fail", name, ex)
            continue
        vm = vermagic_of(dest)
        print(" ", vm[:80], flush=True)
        if MATCH not in vm:
            print("  SKIP wrong vermagic", flush=True)
            dest.unlink()

    syms = {}
    for ko in sorted(KOS.glob("*.ko")):
        s = parse_ko(ko)
        print(ko.name, len(s), flush=True)
        # later files override — all should be same CRC for same kernel
        for n, c in s.items():
            if n in syms and syms[n] != c:
                print("CRC CONFLICT", n, hex(syms[n]), hex(c))
            syms[n] = c

    needed = ["_printk", "ioremap_prot", "iounmap", "param_ops_int",
              "arm64_use_ng_mappings", "log_write_mmio", "log_post_write_mmio", "module_layout"]
    for n in needed:
        print("CRC", n, hex(syms[n]) if n in syms else "MISSING")

    lines = [f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for n, c in sorted(syms.items())]
    for n in needed:
        if n not in syms:
            lines.append(f"0x00000000\t{n}\tvmlinux\tEXPORT_SYMBOL\t\n")
    (K / "Module.symvers").write_text("".join(lines))

    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    cmd = ["make", f"-C{K}", f"M={BUILD}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
           f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", "modules"]
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
        v = parse_ko(ko)
        for n in needed:
            if n in v:
                print(f"  ko {n}=0x{v[n]:08x}")
    return r.returncode


if __name__ == "__main__":
    raise SystemExit(main())
