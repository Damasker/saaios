#!/usr/bin/env python3
from pathlib import Path
k = Path("/home/mike/kernel-work/common-bd23337")
cfg = (k / ".config").read_text(errors="ignore")
for key in (
    "CONFIG_KASAN",
    "CONFIG_KASAN_GENERIC",
    "CONFIG_KASAN_SW_TAGS",
    "CONFIG_MODVERSIONS",
    "CONFIG_TRIM_UNUSED_KSYMS",
    "CONFIG_MODULE_FORCE_LOAD",
    "CONFIG_LOCALVERSION",
):
    hits = [ln for ln in cfg.splitlines() if ln.startswith(key) or ln.startswith(f"# {key}")]
    print(key, hits[:5] if hits else "MISSING")
sym = k / "Module.symvers"
print("symvers", sym.exists(), sym.stat().st_size if sym.exists() else 0)
print("prepare_ok", (k / "include/config/kernel.release").read_text().strip() if (k / "include/config/kernel.release").exists() else None)
