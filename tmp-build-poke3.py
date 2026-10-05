#!/usr/bin/env python3
"""Rebuild poke.ko with CRCs extracted via objcopy (correct endian)."""
import os, re, shutil, struct, subprocess
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects/saaios-som")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = ROOT / "os/targets/panther/diagnostics"
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS_DIR = Path("/home/mike/kernel-work/phone-kos")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"

NEEDED = [
    "_printk",
    "ioremap_prot",
    "iounmap",
    "param_ops_int",
    "arm64_use_ng_mappings",
    "log_write_mmio",
    "log_post_write_mmio",
    "module_layout",
]


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


def parse_versions_ko(path: Path) -> dict[str, int]:
    raw = KOS_DIR / (path.name + ".versions.bin")
    subprocess.run(
        ["objcopy", "-O", "binary", "-j", "__versions", str(path), str(raw)],
        check=True,
        capture_output=True,
    )
    data = raw.read_bytes()
    syms = {}
    # 64-byte records: u64 crc + 56-byte name
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if not name_b or not all(32 <= c < 127 for c in name_b):
            continue
        name = name_b.decode("ascii")
        if re.fullmatch(r"[A-Za-z0-9_]+", name):
            syms[name] = crc
    return syms


def main():
    syms: dict[str, int] = {}
    for ko in sorted(KOS_DIR.glob("*.ko")):
        try:
            s = parse_versions_ko(ko)
            print(ko.name, len(s), flush=True)
            syms.update(s)
        except Exception as ex:
            print("fail", ko.name, ex, flush=True)
    print("unique", len(syms), flush=True)

    lines = []
    missing = []
    # Prefer full dump for correctness of any transitive deps, but sanitize.
    for name, crc in sorted(syms.items()):
        lines.append(f"0x{crc:08x}\t{name}\tvmlinux\tEXPORT_SYMBOL_GPL\n")
    for n in NEEDED:
        if n not in syms:
            missing.append(n)
            lines.append(f"0x00000000\t{n}\tvmlinux\tEXPORT_SYMBOL\n")
        else:
            print(f"CRC {n}=0x{syms[n]:08x}", flush=True)
    if missing:
        print("MISSING", missing, flush=True)

    # Verify _printk matches phone dump 0xd87e9992
    print("printk check", hex(syms.get("_printk", 0)), "expect 0xd87e9992", flush=True)

    (K / "Module.symvers").write_text("".join(lines))
    print("symvers lines", len(lines), flush=True)

    BUILD.mkdir(parents=True, exist_ok=True)
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
    for pat in ("*.o", "*.ko", "*.mod", "*.mod.c", "*.cmd", "Module.symvers", "modules.order"):
        for p in BUILD.glob(pat):
            p.unlink()
    # also .*.cmd
    for p in BUILD.glob(".*"):
        if p.is_file():
            p.unlink()

    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    cmd = [
        "make", f"-C{K}", f"M={BUILD}",
        "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}",
        f"HOSTCFLAGS={host_inc}",
        f"HOSTLDFLAGS={host_ld}",
        "modules",
    ]
    print("RUN", cmd, flush=True)
    r = subprocess.run(cmd, cwd=BUILD, env=env_for_make())
    ko = BUILD / "saaios_cp_poke.ko"
    print("EXIT", r.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
    if ko.exists():
        out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
        for line in out.splitlines():
            if "vermagic=" in line:
                print(line)
                break
        vers = parse_versions_ko(ko)
        for n in NEEDED:
            if n in vers:
                print(f"  ko {n}=0x{vers[n]:08x}")
    return r.returncode


if __name__ == "__main__":
    raise SystemExit(main())
