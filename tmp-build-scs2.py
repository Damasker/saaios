#!/usr/bin/env python3
from pathlib import Path
import subprocess, os, re, shutil, struct

DEB = Path.home() / "local/debroot"
K = Path("/home/mike/kernel-work/common-bd23337")
SRC = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics")
BUILD = Path("/home/mike/kernel-work/saaios_cp_poke_build")
KOS = Path("/home/mike/kernel-work/phone-kos")
REL = "6.1.157-android14-11-gbd23337e42e7-ab14791245"
OBJCOPY = "aarch64-linux-gnu-objcopy"

env = os.environ.copy()
env["PATH"] = f"{DEB/'usr/bin'}:{env.get('PATH','')}"
env["LD_LIBRARY_PATH"] = f"{DEB/'usr/lib/x86_64-linux-gnu'}:{DEB/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"
env["BISON_PKGDATADIR"] = str(DEB / "usr/share/bison")
env["M4"] = str(DEB / "usr/bin/m4")
host_inc = f"-I{DEB/'usr/include'} -I{DEB/'usr/include/x86_64-linux-gnu'}"
host_ld = f"-L{DEB/'usr/lib/x86_64-linux-gnu'}"

# compiler SCS probe
r = subprocess.run(
    ["aarch64-linux-gnu-gcc", "-fsanitize=shadow-call-stack", "-c", "-x", "c", "-o", "/tmp/scs.o", "-"],
    input=b"void f(void){}\n", capture_output=True,
)
print("gcc SCS", r.returncode, r.stderr[-200:].decode(errors="ignore"))

cfg = K / ".config"
text = cfg.read_text()
# force SCS on, KASAN/BTF off, CFI off for GCC build (clang CFI unsupported)
repls = {
    "# CONFIG_SHADOW_CALL_STACK is not set": "CONFIG_SHADOW_CALL_STACK=y",
    "# CONFIG_CFI_CLANG is not set": "# CONFIG_CFI_CLANG is not set",
}
if "CONFIG_SHADOW_CALL_STACK=y" not in text:
    if "# CONFIG_SHADOW_CALL_STACK is not set" in text:
        text = text.replace("# CONFIG_SHADOW_CALL_STACK is not set", "CONFIG_SHADOW_CALL_STACK=y")
    else:
        text += "\nCONFIG_SHADOW_CALL_STACK=y\n"
text = text.replace("CONFIG_KASAN=y", "# CONFIG_KASAN is not set")
text = text.replace("CONFIG_DEBUG_INFO_BTF=y", "# CONFIG_DEBUG_INFO_BTF is not set")
text = text.replace("CONFIG_CFI_CLANG=y", "# CONFIG_CFI_CLANG is not set")
cfg.write_text(text)

for target in ("olddefconfig", "modules_prepare"):
    cmd = ["make", f"-C{K}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
           f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", target]
    print("RUN", target, flush=True)
    rr = subprocess.run(cmd, env=env)
    print(target, rr.returncode, flush=True)
    if rr.returncode != 0:
        raise SystemExit(rr.returncode)

cfg_txt = cfg.read_text()
for key in ("CONFIG_SHADOW_CALL_STACK", "CONFIG_CFI_CLANG", "CONFIG_KASAN", "CONFIG_DEBUG_INFO_BTF"):
    hits = [ln for ln in cfg_txt.splitlines() if ln.startswith(key) or ln.startswith("# " + key)]
    print(key, hits[:3])

# symvers from phone kos
syms = {}
for ko in sorted(KOS.glob("*.ko")):
    raw = Path("/tmp") / (ko.name + ".ver")
    subprocess.run([OBJCOPY, "-O", "binary", "-j", "__versions", str(ko), str(raw)], check=True, capture_output=True)
    data = raw.read_bytes()
    for off in range(0, len(data)//64*64, 64):
        crc = struct.unpack_from("<Q", data, off)[0] & 0xFFFFFFFF
        name_b = data[off+8:off+64].split(b"\x00", 1)[0]
        if name_b and re.fullmatch(rb"[A-Za-z0-9_]+", name_b):
            syms[name_b.decode()] = crc
lines = [f"0x{c:08x}\t{n}\tvmlinux\tEXPORT_SYMBOL_GPL\t\n" for n,c in sorted(syms.items())]
(K / "Module.symvers").write_text("".join(lines))
print("symvers", len(lines))

if BUILD.exists():
    shutil.rmtree(BUILD)
BUILD.mkdir()
shutil.copy2(SRC / "saaios_cp_poke.c", BUILD / "saaios_cp_poke.c")
(BUILD / "Makefile").write_text("obj-m += saaios_cp_poke.o\n")
cmd = ["make", f"-C{K}", f"M={BUILD}", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-",
       f"KERNELRELEASE={REL}", f"HOSTCFLAGS={host_inc}", f"HOSTLDFLAGS={host_ld}", "modules"]
print("RUN modules", flush=True)
rr = subprocess.run(cmd, cwd=BUILD, env=env)
ko = BUILD / "saaios_cp_poke.ko"
print("EXIT", rr.returncode, "ko", ko.exists(), ko.stat().st_size if ko.exists() else 0)
if ko.exists():
    shutil.copy2(ko, SRC / "saaios_cp_poke.ko")
    # show if SCS reloc/note present
    out = subprocess.check_output(["readelf", "-n", str(ko)], text=True, errors="ignore")
    print("notes", "shadow" in out.lower(), out[:500])
    out2 = subprocess.check_output(["strings", str(ko)], text=True, errors="ignore")
    for line in out2.splitlines():
        if "vermagic=" in line:
            print(line)
            break
raise SystemExit(rr.returncode)
