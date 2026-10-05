#!/usr/bin/env python3
"""Rebuild poke with phone-config KASAN layouts + clang CFI/SCS; no KASAN instrument on .o."""
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

POKE_C = (SRC / "saaios_cp_poke.c").read_text()  # keep current source


def env_for_make():
    e = os.environ.copy()
    e["PATH"] = f"{CLANG}:{DEB/'usr/bin'}:{e.get('PATH','')}"
    e["LD_LIBRARY_PATH"] = f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:{e.get('LD_LIBRARY_PATH','')}"
    e["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
    e["M4"] = str(DEB / "usr/bin/m4")
    return e


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
    # Restore phone config almost verbatim
    phone = gzip.open(PHONE_CFG, "rt").read()
    (K / ".config").write_text(phone)
    subprocess.run([
        "scripts/config",
        "--disable", "LOCALVERSION_AUTO",
        "--set-str", "LOCALVERSION", "-android14-11-gbd23337e42e7-ab14791245",
        "--disable", "DEBUG_INFO_BTF",
        "--disable", "DEBUG_INFO_BTF_MODULES",
        "--disable", "TRIM_UNUSED_KSYMS",
    ], cwd=K, check=False)

    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    env = env_for_make()
    cc = str(CLANG / "clang")
    ld = str(CLANG / "ld.lld")
    base = [
        "make", f"-C{K}", "ARCH=arm64",
        f"CC={cc}", f"LD={ld}", "LLVM=1", "LLVM_IAS=1",
        "CLANG_TRIPLE=aarch64-linux-gnu-", "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}",
    ]
    for target in ("olddefconfig", "modules_prepare"):
        print("RUN", target, flush=True)
        r = subprocess.run(base + [target], env=env)
        if r.returncode:
            return r.returncode
    cfg = (K / ".config").read_text()
    for key in ("CONFIG_KASAN=", "CONFIG_SHADOW_CALL_STACK", "CONFIG_CFI_CLANG", "CONFIG_ARM64_BTI_KERNEL"):
        print([ln for ln in cfg.splitlines() if ln.startswith(key) or ln.startswith("# " + key.split("=")[0])][:3])

    # canary offset from generated headers
    for p in (K / "include/generated/asm-offsets.h", K / "include/generated/asm-offsets.s"):
        if p.exists():
            t = p.read_text(errors="ignore")
            for ln in t.splitlines():
                if "TSK_STACK_CANARY" in ln or "stack_canary" in ln.lower():
                    print("OFFSET", ln)

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
    needed = ["_printk", "ioremap_prot", "iounmap", "param_ops_int", "param_ops_ulong",
              "arm64_use_ng_mappings", "log_write_mmio", "log_post_write_mmio", "module_layout"]
    for n in needed:
        print("CRC", n, hex(syms[n]) if n in syms else "MISSING")
    lines = [f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for n, c in sorted(syms.items())]
    (K / "Module.symvers").write_text("".join(lines))

    if BUILD.exists():
        shutil.rmtree(BUILD)
    BUILD.mkdir()
    shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
    # Disable KASAN instrumentation on this unit only; keep phone layout from KASAN=y config
    (BUILD / "Makefile").write_text(
        "obj-m += saaios_cp_poke.o\n"
        "CFLAGS_saaios_cp_poke.o += -fno-sanitize=kernel-address\n"
        "CFLAGS_saaios_cp_poke.mod.o += -fno-sanitize=kernel-address\n"
    )
    cmd = base + [f"M={BUILD}", "modules"]
    print("RUN modules", flush=True)
    r = subprocess.run(cmd, cwd=BUILD, env=env)
    ko = BUILD / "saaios_cp_poke.ko"
    print("EXIT", r.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
    if ko.exists():
        shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
        out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
        for line in out.splitlines():
            if "vermagic=" in line:
                print(line)
                break
        cmdf = (BUILD / ".saaios_cp_poke.o.cmd").read_text()
        print("has kasan instrument", "kernel-address" in cmdf and "fno-sanitize=kernel-address" not in cmdf)
        print("has fno-sanitize=kernel-address", "fno-sanitize=kernel-address" in cmdf)
        print("has kcfi", "kcfi" in cmdf)
        print("has scs", "shadow-call-stack" in cmdf)
        m = re.search(r"mstack-protector-guard-offset=(\d+)", cmdf)
        print("canary_off", m.group(1) if m else None)
    return r.returncode


if __name__ == "__main__":
    raise SystemExit(main())
