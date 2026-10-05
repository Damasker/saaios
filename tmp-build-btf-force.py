#!/usr/bin/env python3
"""Force CONFIG_DEBUG_INFO_BTF_MODULES into prepared tree; rebuild poke; check 0x440.

Source of truth for sizeof(struct module)==0x440:
  phone /proc/config.gz → CONFIG_DEBUG_INFO_BTF_MODULES=y (ADR-024;
  phone-config.gz in diagnostics; kernel/common@bd23337e42e7 ab14791245).
BTF_MODULES adds btf_data{,_size}; ____cacheline_aligned rounds 0x400→0x440.
"""
from __future__ import annotations

import gzip
import os
import re
import shutil
import struct
import subprocess
from pathlib import Path

CLANG = Path("/home/mike/local/clang/r487747c/bin")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS_MATCH = Path("/home/mike/kernel-work/phone-kos-match")
KOS_ANY = Path("/home/mike/kernel-work/phone-kos")
PHONE_CFG = SRC / "phone-config.gz"
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"
STOCK = KOS_MATCH / "rfkill.ko"


def env_for_make() -> dict:
    e = os.environ.copy()
    e["PATH"] = f"{CLANG}:{DEB / 'usr/bin'}:{e.get('PATH', '')}"
    e["LD_LIBRARY_PATH"] = (
        f"{DEB / 'usr/lib/x86_64-linux-gnu'}:{DEB / 'lib/x86_64-linux-gnu'}:"
        f"{e.get('LD_LIBRARY_PATH', '')}"
    )
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    # Skip real pahole/resolve during module BTF step if invoked
    e["PAHOLE"] = "true"
    return e


def this_module_size(ko: Path) -> int:
    out = subprocess.check_output(
        [str(CLANG / "llvm-objdump"), "-h", str(ko)], text=True, errors="ignore"
    )
    for ln in out.splitlines():
        if ".gnu.linkonce.this_module" in ln and "rela" not in ln:
            m = re.search(r"this_module\s+([0-9a-fA-F]{8,})", ln)
            if m:
                return int(m.group(1), 16)
            parts = ln.split()
            for p in parts:
                if re.fullmatch(r"[0-9a-fA-F]{8}", p):
                    return int(p, 16)
    raise RuntimeError(f"no this_module in {ko}\n{out}")


def parse_ko(path: Path) -> dict[str, int]:
    raw = Path("/tmp") / (path.name + ".ver")
    subprocess.run(
        [OBJCOPY, "-O", "binary", "-j", "__versions", str(path), str(raw)],
        check=True,
        capture_output=True,
    )
    data = raw.read_bytes()
    syms: dict[str, int] = {}
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
    return syms


def force_btf_modules() -> None:
    """Inject BTF_MODULES like phone without building resolve_btfids/pahole."""
    # .config
    cfg_path = K / ".config"
    lines = cfg_path.read_text().splitlines()
    out = []
    seen = set()
    for ln in lines:
        if ln.startswith("CONFIG_DEBUG_INFO_BTF_MODULES") or ln.startswith(
            "# CONFIG_DEBUG_INFO_BTF_MODULES"
        ):
            continue
        if ln.startswith("CONFIG_MODULE_SCMVERSION") or ln.startswith(
            "# CONFIG_MODULE_SCMVERSION"
        ):
            continue
        if ln.startswith("CONFIG_MODULE_ALLOW_BTF_MISMATCH") or ln.startswith(
            "# CONFIG_MODULE_ALLOW_BTF_MISMATCH"
        ):
            continue
        out.append(ln)
    out.append("CONFIG_DEBUG_INFO_BTF=y")
    out.append("CONFIG_DEBUG_INFO_BTF_MODULES=y")
    out.append("CONFIG_MODULE_SCMVERSION=y")
    out.append("CONFIG_MODULE_ALLOW_BTF_MISMATCH=y")
    cfg_path.write_text("\n".join(out) + "\n")

    # include/config/auto.conf
    ac = K / "include/config/auto.conf"
    if ac.exists():
        text = ac.read_text()
        for key in (
            "CONFIG_DEBUG_INFO_BTF=y",
            "CONFIG_DEBUG_INFO_BTF_MODULES=y",
            "CONFIG_MODULE_SCMVERSION=y",
            "CONFIG_MODULE_ALLOW_BTF_MISMATCH=y",
        ):
            name = key.split("=")[0]
            text = re.sub(rf"(?m)^{name}=.*\n", "", text)
            text = re.sub(rf"(?m)^# {name} is not set\n", "", text)
            text += key + "\n"
        ac.write_text(text)

    # include/generated/autoconf.h
    ah = K / "include/generated/autoconf.h"
    text = ah.read_text()
    for name in (
        "CONFIG_DEBUG_INFO_BTF",
        "CONFIG_DEBUG_INFO_BTF_MODULES",
        "CONFIG_MODULE_SCMVERSION",
        "CONFIG_MODULE_ALLOW_BTF_MISMATCH",
    ):
        text = re.sub(rf"(?m)^#define {name} .*\n", "", text)
        text = re.sub(rf"(?m)^/\* {name} is not set \*/\n", "", text)
        text += f"#define {name} 1\n"
    ah.write_text(text)

    # compile.h / autoconf for rust not needed
    print("forced BTF_MODULES into .config/auto.conf/autoconf.h", flush=True)


def dump_sizeof() -> None:
    """Compile a tiny TU printing offsetof/sizeof under current headers."""
    stub = Path("/tmp/dump_mod_size.c")
    stub.write_text(
        """
#define __KERNEL__
#define MODULE
#include <linux/module.h>
unsigned long saaios_module_size = sizeof(struct module);
unsigned long saaios_btf_off =
#ifdef CONFIG_DEBUG_INFO_BTF_MODULES
    __builtin_offsetof(struct module, btf_data);
#else
    0xdead;
#endif
"""
    )
    # Just preprocess / compile to object and nm the absolute? Better: use clang -E
    # Actually sizeof needs full compile. Use a host-targeted dump via sparse? No —
    # cross-compile to .o and read via llvm-nm after embedding as global.
    # Simpler: compile with -c and llvm-objdump -t / read from .o size of common symbol.
    pass


def make_base() -> list[str]:
    host_inc = f"-I{DEB / 'usr/include'} -I{DEB / 'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB / 'usr/lib/x86_64-linux-gnu'}"
    cc, ld = str(CLANG / "clang"), str(CLANG / "ld.lld")
    return [
        "make",
        f"-C{K}",
        "ARCH=arm64",
        f"CC={cc}",
        f"LD={ld}",
        "LLVM=1",
        "LLVM_IAS=1",
        "CLANG_TRIPLE=aarch64-linux-gnu-",
        "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}",
        f"HOSTCFLAGS={host_inc}",
        f"HOSTLDFLAGS={host_ld}",
        "PAHOLE=true",
    ]


def ensure_prepare(env: dict) -> None:
    """modules_prepare with BTF tools stubbed so BTF can stay on in config."""
    phone = gzip.open(PHONE_CFG, "rt").read()
    (K / ".config").write_text(phone)
    # Disable ALL BTF during prepare — resolve_btfids host tool fails under clang-17
    # (const discard / missing libbpf). Layout is restored via force_btf_modules() after.
    subprocess.run(
        [
            "scripts/config",
            "--disable",
            "LOCALVERSION_AUTO",
            "--set-str",
            "LOCALVERSION",
            "-android14-11-gbd23337e42e7-ab14791245",
            "--disable",
            "DEBUG_INFO_BTF",
            "--disable",
            "DEBUG_INFO_BTF_MODULES",
            "--disable",
            "TRIM_UNUSED_KSYMS",
            "--enable",
            "MODULE_SCMVERSION",
        ],
        cwd=K,
        check=False,
    )
    base = make_base()
    print("RUN olddefconfig", flush=True)
    subprocess.run(base + ["olddefconfig"], env=env, check=True)

    print("RUN modules_prepare", flush=True)
    r = subprocess.run(base + ["modules_prepare"], env=env)
    if r.returncode != 0:
        raise SystemExit(f"modules_prepare failed: {r.returncode}")


def rebuild_symvers() -> None:
    syms: dict[str, int] = {}
    for d in (KOS_ANY, KOS_MATCH):
        if not d.exists():
            continue
        for ko in sorted(d.glob("*.ko")):
            try:
                s = parse_ko(ko)
            except Exception:
                continue
            if d == KOS_MATCH:
                syms.update(s)
            else:
                for n, c in s.items():
                    syms.setdefault(n, c)
    lines = [
        f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n"
        for n, c in sorted(syms.items())
    ]
    (K / "Module.symvers").write_text("".join(lines))
    print(f"Module.symvers entries={len(lines)}", flush=True)


def compile_sizeof(env: dict) -> int:
    """Cross-compile a TU that exports sizeof(struct module) as a u64 absolute."""
    tdir = Path("/tmp/saaios_mod_sizeof")
    if tdir.exists():
        shutil.rmtree(tdir)
    tdir.mkdir()
    (tdir / "sz.c").write_text(
        "#include <linux/module.h>\n"
        "#include <linux/init.h>\n"
        "unsigned long long saaios_sz = sizeof(struct module);\n"
        "MODULE_LICENSE(\"GPL\");\n"
    )
    (tdir / "Makefile").write_text("obj-m += sz.o\n")
    base = make_base()
    r = subprocess.run(base + [f"M={tdir}", "modules"], cwd=tdir, env=env)
    if r.returncode != 0:
        print("sizeof helper build failed", r.returncode, flush=True)
        return -1
    # Read from .o via llvm-objdump -s -j .data
    o = tdir / "sz.o"
    out = subprocess.check_output(
        [str(CLANG / "llvm-objdump"), "-s", "-j", ".data", str(o)],
        text=True,
        errors="ignore",
    )
    print("sz.o .data:\n", out, flush=True)
    # Also this_module from sz.ko
    ko = tdir / "sz.ko"
    if ko.exists():
        return this_module_size(ko)
    return -1


def main() -> int:
    env = env_for_make()
    ensure_prepare(env)
    force_btf_modules()
    rebuild_symvers()

    # Confirm autoconf
    ah = (K / "include/generated/autoconf.h").read_text()
    assert "CONFIG_DEBUG_INFO_BTF_MODULES 1" in ah.replace("\t", " ")
    print("autoconf OK BTF_MODULES", flush=True)

    sz = compile_sizeof(env)
    print(f"helper this_module size=0x{sz:x}" if sz > 0 else "helper failed", flush=True)

    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text(
        "obj-m += saaios_cp_poke.o\n"
        "CFLAGS_saaios_cp_poke.o += -fno-sanitize=kernel-address\n"
        "CFLAGS_saaios_cp_poke.mod.o += -fno-sanitize=kernel-address\n"
    )
    print("RUN poke modules", flush=True)
    r = subprocess.run(make_base() + [f"M={BUILD}", "modules"], cwd=BUILD, env=env)
    ko = BUILD / "saaios_cp_poke.ko"
    print("EXIT", r.returncode, "exists", ko.exists(), flush=True)
    if not ko.exists():
        # show build log hint
        return r.returncode or 1

    ours = this_module_size(ko)
    stock = this_module_size(STOCK) if STOCK.exists() else -1
    print(f"this_module ours=0x{ours:x} stock=0x{stock:x} match={ours == stock}", flush=True)
    shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
    for line in subprocess.check_output(["strings", str(ko)], text=True, errors="ignore").splitlines():
        if "vermagic=" in line:
            print(line, flush=True)
            break
    # module_layout CRC if present
    try:
        vers = parse_ko(ko)
        print("module_layout CRC", hex(vers.get("module_layout", 0)), flush=True)
        if STOCK.exists():
            svers = parse_ko(STOCK)
            print("stock module_layout CRC", hex(svers.get("module_layout", 0)), flush=True)
    except Exception as e:
        print("crc parse", e, flush=True)
    return 0 if ours == stock else 2


if __name__ == "__main__":
    raise SystemExit(main())
