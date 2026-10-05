#!/usr/bin/env python3
"""Pull phone .ko __versions, build Module.symvers, compile saaios_cp_poke.ko."""
import os, re, shutil, struct, subprocess, urllib.request
from pathlib import Path

ROOT = Path("/mnt/c/Users/Admin/Projects/saaios-som")
PHONE = "http://172.31.7.1:8080"
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = ROOT / "os/targets/panther/diagnostics"
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS_DIR = Path("/home/mike/kernel-work/phone-kos")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"

NEEDED = {
    "_printk",
    "ioremap_prot",
    "iounmap",
    "param_ops_int",
    "arm64_use_ng_mappings",
    "log_write_mmio",
    "log_post_write_mmio",
    "__asan_load1_noabort",
    "__asan_store1_noabort",
    "__asan_load2_noabort",
    "__asan_store2_noabort",
    "__asan_load4_noabort",
    "__asan_store4_noabort",
    "__asan_load8_noabort",
    "__asan_store8_noabort",
    "__asan_loadN_noabort",
    "__asan_storeN_noabort",
    "__asan_register_globals",
    "__asan_unregister_globals",
    "__asan_handle_no_return",
    "module_layout",
}

# Prefer modules known to carry ioremap / params; then sample more.
FETCH = [
    "cpif.ko",
    "clk_exynos_gs.ko",
    "cmupmucal.ko",
    "bcmdhd4389.ko",
    "pcie_exynos_gs.ko",
    "eh.ko",
    "exynos-drm.ko",
    "google_bcl.ko",
    "cfg80211.ko",
    "g2d.ko",
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


def parse_versions(data: bytes) -> dict[str, int]:
    """Parse ELF __versions section: 64-byte records (u64 crc + 56-byte name)."""
    # Find section via readelf-like scan: look for PROGBITS named in ELF is hard;
    # instead scan for known pattern from readelf -x already pulled, OR use pyelf.
    # Use simple: invoke readelf -x __versions
    return {}


def parse_versions_file(path: Path) -> dict[str, int]:
    out = subprocess.check_output(["readelf", "-x", "__versions", str(path)], text=True, errors="ignore")
    hexbytes = bytearray()
    for line in out.splitlines():
        # "  0x00000000 3a434c89 00000000 5f5f706c 6174666f ...."
        m = re.match(r"\s*0x[0-9a-fA-F]+\s+((?:[0-9a-fA-F]{8}\s*){1,4})", line)
        if not m:
            continue
        for w in m.group(1).split():
            hexbytes.extend(bytes.fromhex(w))
    syms = {}
    for off in range(0, len(hexbytes) - 8, 64):
        crc = struct.unpack_from("<Q", hexbytes, off)[0] & 0xFFFFFFFF
        name = hexbytes[off + 8 : off + 64].split(b"\x00", 1)[0].decode("ascii", "ignore")
        if name:
            syms[name] = crc
    return syms


def fetch_kos():
    KOS_DIR.mkdir(parents=True, exist_ok=True)
    got = []
    for name in FETCH:
        dest = KOS_DIR / name
        if dest.exists() and dest.stat().st_size > 1000:
            got.append(dest)
            continue
        url = f"{PHONE}/{name}"
        print("GET", url, flush=True)
        try:
            urllib.request.urlretrieve(url, dest)
            got.append(dest)
        except Exception as ex:
            print("FAIL", name, ex, flush=True)
    return got


def disable_kasan():
    """Avoid ASAN/MMIO-trace deps in OOT module; keep MODVERSIONS."""
    cfg = K / ".config"
    text = cfg.read_text()
    reps = [
        ("CONFIG_KASAN=y", "# CONFIG_KASAN is not set"),
        ("CONFIG_KASAN_GENERIC=y", "# CONFIG_KASAN_GENERIC is not set"),
        ("CONFIG_KASAN_INLINE=y", "# CONFIG_KASAN_INLINE is not set"),
        ("CONFIG_KASAN_OUTLINE=y", "# CONFIG_KASAN_OUTLINE is not set"),
        ("CONFIG_KASAN_STACK=y", "# CONFIG_KASAN_STACK is not set"),
        ("CONFIG_SHADOW_CALL_STACK=y", "# CONFIG_SHADOW_CALL_STACK is not set"),
    ]
    for a, b in reps:
        text = text.replace(a, b)
    # MMIO write logging pulls log_write_mmio
    for key in (
        "CONFIG_TRACE_MMIO_ACCESS",
        "CONFIG_FTRACE_MMIO",
    ):
        text = re.sub(rf"^{key}=y$", f"# {key} is not set", text, flags=re.M)
    cfg.write_text(text)
    # Refresh autoconf for CC_FLAGS_KASAN etc.
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    cmd = [
        "make", f"-C{K}",
        "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}",
        f"HOSTCFLAGS={host_inc}",
        f"HOSTLDFLAGS={host_ld}",
        "olddefconfig",
    ]
    print("RUN", cmd[:6], "... olddefconfig", flush=True)
    subprocess.run(cmd, env=env_for_make(), check=False)
    cmd[-1] = "modules_prepare"
    print("RUN modules_prepare", flush=True)
    r = subprocess.run(cmd, env=env_for_make())
    print("prepare", r.returncode, flush=True)
    return r.returncode


def write_symvers(syms: dict[str, int]):
    lines = []
    for name, crc in sorted(syms.items()):
        lines.append(f"0x{crc:08x}\t{name}\tvmlinux\tEXPORT_SYMBOL_GPL\n")
    # Ensure needed symbols present (CRC 0 placeholder last resort)
    for n in NEEDED:
        if n not in syms:
            lines.append(f"0x00000000\t{n}\tvmlinux\tEXPORT_SYMBOL\n")
            print("WARN missing CRC for", n, flush=True)
    path = K / "Module.symvers"
    path.write_text("".join(lines))
    print("Wrote", path, "entries", len(lines), flush=True)
    for n in sorted(NEEDED):
        if n in syms:
            print(f"  CRC {n}=0x{syms[n]:08x}", flush=True)


def build_ko():
    BUILD.mkdir(parents=True, exist_ok=True)
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
    # wipe stale
    for p in BUILD.glob("*.o"):
        p.unlink()
    for p in BUILD.glob("*.ko"):
        p.unlink()
    for p in BUILD.glob("*.mod*"):
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
    print("EXIT", r.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0, flush=True)
    if ko.exists():
        out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
        for line in out.splitlines():
            if "vermagic=" in line:
                print(line, flush=True)
                break
        # also dump our __versions
        try:
            vers = parse_versions_file(ko)
            print("ko versions", len(vers), flush=True)
            for n in sorted(NEEDED):
                if n in vers:
                    print(f"  ko {n}=0x{vers[n]:08x}", flush=True)
        except Exception as ex:
            print("versions parse", ex, flush=True)
    return r.returncode


def main():
    kos = fetch_kos()
    syms: dict[str, int] = {}
    for ko in kos:
        try:
            s = parse_versions_file(ko)
            print(ko.name, "versions", len(s), flush=True)
            syms.update(s)
        except Exception as ex:
            print("parse fail", ko, ex, flush=True)
    print("total unique symbols", len(syms), flush=True)
    write_symvers(syms)
    rc = disable_kasan()
    if rc != 0:
        print("prepare failed", rc)
        return rc
    return build_ko()


if __name__ == "__main__":
    raise SystemExit(main())
