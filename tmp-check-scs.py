#!/usr/bin/env python3
import os
from pathlib import Path
p = Path("/home/mike/kernel-work/common-bd23337/.config")
t = p.read_text()
for k in ("CONFIG_SHADOW_CALL_STACK", "CONFIG_CFI_CLANG", "CONFIG_KASAN", "CONFIG_MODVERSIONS", "CONFIG_LTO"):
    for ln in t.splitlines():
        if ln.startswith(k) or ln.startswith(f"# {k}"):
            print(ln)
