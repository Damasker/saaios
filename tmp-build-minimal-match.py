#!/usr/bin/env python3
from pathlib import Path
import struct, re, subprocess, shutil, os

DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS = Path("/home/mike/kernel-work/phone-kos-match")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"

# Minimal module — only printk + int param — prove load works with matching CRCs+SCS
(SRC / "saaios_cp_poke.c").write_text(r'''#include <linux/module.h>
static int dry_run = 1;
module_param(dry_run, int, 0644);
static int __init saaios_cp_poke_init(void)
{
	pr_info("saaios_cp_poke: MINIMAL load OK dry=%d\n", dry_run);
	return 0;
}
static void __exit saaios_cp_poke_exit(void)
{
	pr_info("saaios_cp_poke: exit\n");
}
module_init(saaios_cp_poke_init);
module_exit(saaios_cp_poke_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("SaaiOS poke minimal load test");
MODULE_AUTHOR("SaaiOS");
''')

syms = {}
for ko in sorted(KOS.glob("*.ko")):
    raw = Path("/tmp") / (ko.name + ".ver")
    subprocess.run([OBJCOPY, "-O", "binary", "-j", "__versions", str(ko), str(raw)],
                   check=True, capture_output=True)
    data = raw.read_bytes()
    for off in range(0, len(data) // 64 * 64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off + 8 : off + 64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
    print(ko.name, "ok", flush=True)

for n in ("_printk", "param_ops_int", "module_layout", "param_ops_ulong"):
    print(n, hex(syms[n]) if n in syms else "MISSING")

lines = [f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for n, c in sorted(syms.items())]
(K / "Module.symvers").write_text("".join(lines))

env = os.environ.copy()
env["PATH"] = f"{DEB/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["M4"] = str(DEB / "usr/bin/m4")
env["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"

if BUILD.exists():
    shutil.rmtree(BUILD)
BUILD.mkdir()
shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
(BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
cmd = ["make", f"-C{K}", f"M={BUILD}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
       f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", "modules"]
r = subprocess.run(cmd, cwd=BUILD, env=env)
ko = BUILD / "saaios_cp_poke.ko"
print("EXIT", r.returncode, ko.exists(), ko.stat().st_size if ko.exists() else 0)
if ko.exists():
    shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
    out = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
    for line in out.splitlines():
        if "vermagic=" in line:
            print(line)
            break
raise SystemExit(r.returncode)
