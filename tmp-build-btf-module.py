#!/usr/bin/env python3
"""Enable DEBUG_INFO_BTF_MODULES like phone; rebuild poke; check this_module size."""
import gzip, os, re, shutil, struct, subprocess
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


def env_for_make():
    e = os.environ.copy()
    e["PATH"] = f"{CLANG}:{DEB/'usr/bin'}:{e.get('PATH','')}"
    e["LD_LIBRARY_PATH"] = (
        f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:"
        f"{e.get('LD_LIBRARY_PATH','')}"
    )
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    return e


def this_module_size(ko: Path) -> int:
    out = subprocess.check_output(
        [str(CLANG / "llvm-objdump"), "-h", str(ko)], text=True, errors="ignore"
    )
    for ln in out.splitlines():
        if "this_module" in ln and "rela" not in ln:
            # " 32 .gnu.linkonce.this_module      00000400 ..."
            parts = ln.split()
            for i, p in enumerate(parts):
                if p.startswith("0000") and i + 1 < len(parts):
                    try:
                        return int(p, 16)
                    except ValueError:
                        pass
            m = re.search(r"this_module\s+([0-9a-fA-F]+)", ln)
            if m:
                return int(m.group(1), 16)
    raise RuntimeError(f"no this_module in {ko}")


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


def main():
    # Restore phone config + LOCALVERSION; keep BTF_MODULES ON (ADR-024)
    phone = gzip.open(PHONE_CFG, "rt").read()
    (K / ".config").write_text(phone)
    subprocess.run(
        [
            "scripts/config",
            "--disable", "LOCALVERSION_AUTO",
            "--set-str", "LOCALVERSION", "-android14-11-gbd23337e42e7-ab14791245",
            "--enable", "DEBUG_INFO_BTF",
            "--enable", "DEBUG_INFO_BTF_MODULES",
            "--disable", "TRIM_UNUSED_KSYMS",
        ],
        cwd=K, check=False,
    )

    # Workaround resolve_btfids build: touch stub if prepare fails
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    env = env_for_make()
    cc, ld = str(CLANG / "clang"), str(CLANG / "ld.lld")
    base = [
        "make", f"-C{K}", "ARCH=arm64",
        f"CC={cc}", f"LD={ld}", "LLVM=1", "LLVM_IAS=1",
        "CLANG_TRIPLE=aarch64-linux-gnu-", "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}",
    ]
    for target in ("olddefconfig", "modules_prepare"):
        print("RUN", target, flush=True)
        r = subprocess.run(base + [target], env=env)
        print(target, r.returncode, flush=True)
        if r.returncode != 0 and target == "modules_prepare":
            # Stub resolve_btfids then retry
            stub = K / "tools/bpf/resolve_btfids/resolve_btfids"
            stub.parent.mkdir(parents=True, exist_ok=True)
            stub.write_text("#!/bin/sh\nexit 0\n")
            stub.chmod(0o755)
            # Also try disabling only the tool dependency via PAHOLE
            env2 = env.copy()
            env2["PAHOLE"] = "true"
            r = subprocess.run(base + [target], env=env2)
            print("retry prepare", r.returncode, flush=True)
            if r.returncode != 0:
                return r.returncode

    cfg = (K / ".config").read_text()
    for key in ("CONFIG_DEBUG_INFO_BTF", "CONFIG_DEBUG_INFO_BTF_MODULES", "CONFIG_CFI_CLANG", "CONFIG_KASAN"):
        print([ln for ln in cfg.splitlines() if ln.startswith(key) or ln.startswith("# " + key)][:3])

    # Verify autoconf has BTF_MODULES
    ac = (K / "include/generated/autoconf.h").read_text()
    print("autoconf BTF_MODULES", "CONFIG_DEBUG_INFO_BTF_MODULES 1" in ac.replace("=", " "))

    syms = {}
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
    lines = [f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for n, c in sorted(syms.items())]
    (K / "Module.symvers").write_text("".join(lines))

    # Keep current poke.c (string params version)
    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    (BUILD / "Makefile").write_text(
        "obj-m += saaios_cp_poke.o\n"
        "CFLAGS_saaios_cp_poke.o += -fno-sanitize=kernel-address\n"
        "CFLAGS_saaios_cp_poke.mod.o += -fno-sanitize=kernel-address\n"
    )
    print("RUN modules", flush=True)
    r = subprocess.run(base + [f"M={BUILD}", "modules"], cwd=BUILD, env=env)
    ko = BUILD / "saaios_cp_poke.ko"
    print("EXIT", r.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
    if not ko.exists():
        return r.returncode or 1

    ours = this_module_size(ko)
    stock = this_module_size(STOCK) if STOCK.exists() else -1
    print(f"this_module ours=0x{ours:x} stock=0x{stock:x} match={ours==stock}")
    shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
    out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
    for line in out.splitlines():
        if "vermagic=" in line:
            print(line)
            break
    return 0 if ours == stock else 2


if __name__ == "__main__":
    raise SystemExit(main())
