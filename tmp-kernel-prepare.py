from pathlib import Path
import os, shutil, subprocess, sys

root = Path.home() / "local/debroot"
flex_src = root / "usr/bin/flex"
flex_dst = Path.home() / "local/bin/flex"
flex_dst.parent.mkdir(parents=True, exist_ok=True)
m4_dir = Path("/tmp/u/bin")
m4_dir.mkdir(parents=True, exist_ok=True)
shutil.copy2(root / "usr/bin/m4", m4_dir / "m4")

b = flex_src.read_bytes()
old, new = b"/usr/bin/m4", b"/tmp/u/bin/m4"
print("flex size", len(b), "m4 idx", b.find(old))
if old not in b:
    raise SystemExit("m4 path not found in flex")
flex_dst.write_bytes(b.replace(old, new, 1))
flex_dst.chmod(0o755)
print("patched flex ->", flex_dst)

env = os.environ.copy()
env["PATH"] = f"{Path.home()/'local/bin'}:{root/'usr/bin'}:{env.get('PATH','')}"
env["BISON_PKGDATADIR"] = str(root / "usr/share/bison")
env["LD_LIBRARY_PATH"] = f"{root/'usr/lib/x86_64-linux-gnu'}:{root/'lib/x86_64-linux-gnu'}:{env.get('LD_LIBRARY_PATH','')}"

k = Path("/home/mike/kernel-work/common-bd23337")
cfg = k / ".config"
text = cfg.read_text()
lines = []
for line in text.splitlines():
    if line.startswith("CONFIG_LOCALVERSION="):
        lines.append('CONFIG_LOCALVERSION="-android14-11-gbd23337e42e7-ab14791245"')
    elif line.startswith("CONFIG_LOCALVERSION_AUTO="):
        lines.append("# CONFIG_LOCALVERSION_AUTO is not set")
    else:
        lines.append(line)
cfg.write_text("\n".join(lines) + "\n")

subprocess.check_call(["make", "ARCH=arm64", "olddefconfig"], cwd=k, env=env)
print("OLDDEF_OK")
subprocess.check_call(
    ["make", "ARCH=arm64", "CROSS_COMPILE=aarch64-linux-gnu-", f"-j{os.cpu_count() or 2}", "modules_prepare"],
    cwd=k,
    env=env,
)
print("PREPARE_OK")
print((k / "include/config/kernel.release").read_text().strip())
print("symvers", (k / "Module.symvers").exists(), (k / "Module.symvers").stat().st_size if (k / "Module.symvers").exists() else 0)
