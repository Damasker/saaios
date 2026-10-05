#!/usr/bin/env python3
"""Rebuild saaios_cp_poke.ko with AOSP clang-r487747c + CFI + SCS."""
import os, re, shutil, struct, subprocess
from pathlib import Path

CLANG = Path("/home/mike/local/clang/r487747c/bin")
DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS_MATCH = Path("/home/mike/kernel-work/phone-kos-match")
KOS_ANY = Path("/home/mike/kernel-work/phone-kos")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"

POKE_C = r'''/* CP DRAM poke via PCIe ATU. Built with AOSP clang-r487747c + CFI/SCS. */
#include <linux/module.h>
#include <linux/io.h>

#define SAAIOS_POKE_AP_BASE	0x14200000ul
#define SAAIOS_POKE_MAP_SIZE	SZ_1M

typedef int (*atu_fn_t)(int ch_num, u32 target_addr, u32 offset, u32 size);

static unsigned long atu_fn;
module_param(atu_fn, ulong, 0644);

static int pcie_ch;
module_param(pcie_ch, int, 0644);

static unsigned long cp_phys;
module_param(cp_phys, ulong, 0644);

static int poke_val = -1;
module_param(poke_val, int, 0644);

static int dry_run = 1;
module_param(dry_run, int, 0644);

static int __nocfi do_poke(void)
{
	atu_fn_t fn;
	void __iomem *v;
	u32 atu_off;
	int ret;

	if (!atu_fn || !cp_phys || poke_val < 0 || poke_val > 255)
		return -EINVAL;
	fn = (atu_fn_t)(unsigned long)atu_fn;
	atu_off = (u32)(cp_phys & ~(SAAIOS_POKE_MAP_SIZE - 1));
	pr_info("saaios_cp_poke: ATU ch=%d off=0x%x\n", pcie_ch, atu_off);
	ret = fn(pcie_ch, atu_off, 0, SAAIOS_POKE_MAP_SIZE);
	pr_info("saaios_cp_poke: ATU ret=%d\n", ret);
	if (ret)
		return ret;
	v = ioremap(SAAIOS_POKE_AP_BASE, SAAIOS_POKE_MAP_SIZE);
	if (!v)
		return -ENOMEM;
	writeb((u8)poke_val, v + (cp_phys - atu_off));
	wmb();
	iounmap(v);
	pr_info("saaios_cp_poke: wrote 0x%02x @0x%lx OK\n", poke_val, cp_phys);
	return 0;
}

static int __init saaios_cp_poke_init(void)
{
	pr_info("saaios_cp_poke: init dry=%d atu=%lx phys=%lx val=%d\n",
		dry_run, atu_fn, cp_phys, poke_val);
	if (dry_run) {
		pr_info("saaios_cp_poke: dry_run OK\n");
		return 0;
	}
	return do_poke();
}

static void __exit saaios_cp_poke_exit(void)
{
	pr_info("saaios_cp_poke: exit\n");
}

module_init(saaios_cp_poke_init);
module_exit(saaios_cp_poke_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS CP DRAM poke via PCIe ATU");
MODULE_AUTHOR("SaaiOS");
'''


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


def fix_config():
    cfg = K / ".config"
    text = cfg.read_text()
    # Force SCS + CFI on; KASAN/BTF off for OOT simplicity
    for off in ("CONFIG_KASAN=y", "CONFIG_KASAN_GENERIC=y", "CONFIG_DEBUG_INFO_BTF=y",
                "CONFIG_DEBUG_INFO_BTF_MODULES=y"):
        text = text.replace(off, "# " + off.split("=")[0] + " is not set")
    if "CONFIG_SHADOW_CALL_STACK=y" not in text:
        text = text.replace("# CONFIG_SHADOW_CALL_STACK is not set", "CONFIG_SHADOW_CALL_STACK=y")
        if "CONFIG_SHADOW_CALL_STACK=y" not in text:
            text += "\nCONFIG_SHADOW_CALL_STACK=y\n"
    if "CONFIG_CFI_CLANG=y" not in text:
        text = text.replace("# CONFIG_CFI_CLANG is not set", "CONFIG_CFI_CLANG=y")
        if "CONFIG_CFI_CLANG=y" not in text:
            text += "\nCONFIG_CFI_CLANG=y\n"
    # ensure not permissive
    text = text.replace("CONFIG_CFI_PERMISSIVE=y", "# CONFIG_CFI_PERMISSIVE is not set")
    cfg.write_text(text)
    host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
    host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"
    env = env_for_make()
    cc = str(CLANG / "clang")
    ld = str(CLANG / "ld.lld")
    for target in ("olddefconfig", "modules_prepare"):
        cmd = [
            "make", f"-C{K}", "ARCH=arm64",
            f"CC={cc}", f"LD={ld}", "LLVM=1", "LLVM_IAS=1",
            "CLANG_TRIPLE=aarch64-linux-gnu-",
            "CROSS_COMPILE=aarch64-linux-gnu-",
            f"KERNELRELEASE={REL}",
            f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}",
            target,
        ]
        print("RUN", target, flush=True)
        r = subprocess.run(cmd, env=env)
        print(target, r.returncode, flush=True)
        if r.returncode != 0:
            return r.returncode
    cfg_txt = cfg.read_text()
    for key in ("CONFIG_SHADOW_CALL_STACK", "CONFIG_CFI_CLANG", "CONFIG_CFI_PERMISSIVE", "CONFIG_KASAN"):
        hits = [ln for ln in cfg_txt.splitlines() if ln.startswith(key) or ln.startswith("# " + key)]
        print(key, hits[:3])
    return 0


def main():
    assert (CLANG / "clang").exists(), "clang missing"
    print("clang", subprocess.check_output([str(CLANG / "clang"), "--version"], text=True).splitlines()[0])
    (SRC / "saaios_cp_poke.c").write_text(POKE_C)
    rc = fix_config()
    if rc:
        return rc

    # Prefer matching-vermagic CRCs; fill gaps from any phone kos (core CRCs matched before)
    syms = {}
    for d in (KOS_ANY, KOS_MATCH):
        if not d.exists():
            continue
        for ko in sorted(d.glob("*.ko")):
            try:
                s = parse_ko(ko)
            except Exception as ex:
                print("parse fail", ko.name, ex)
                continue
            # matching dir overrides
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
    cc = str(CLANG / "clang")
    ld = str(CLANG / "ld.lld")
    cmd = [
        "make", f"-C{K}", f"M={BUILD}", "ARCH=arm64",
        f"CC={cc}", f"LD={ld}", "LLVM=1", "LLVM_IAS=1",
        "CLANG_TRIPLE=aarch64-linux-gnu-",
        "CROSS_COMPILE=aarch64-linux-gnu-",
        f"KERNELRELEASE={REL}",
        f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}",
        "modules",
    ]
    print("RUN modules", flush=True)
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
        cmdf = (BUILD / ".saaios_cp_poke.o.cmd").read_text()
        for flag in ("shadow-call-stack", "kcfi", "sanitize=cfi", "fsanitize=cfi", "fixed-x18", "cfi-icall"):
            print(f"flag {flag}:", flag in cmdf)
    return r.returncode


if __name__ == "__main__":
    raise SystemExit(main())
