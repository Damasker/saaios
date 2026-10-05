#!/usr/bin/env python3
from pathlib import Path
import subprocess

k = Path("/home/mike/kernel-work/common-bd23337")
# search ARCH_USES_CFI_TRAPS
for p in k.rglob("Kconfig*"):
    try:
        t = p.read_text(errors="ignore")
    except Exception:
        continue
    if "ARCH_USES_CFI_TRAPS" in t:
        print("===", p)
        for i, ln in enumerate(t.splitlines()):
            if "ARCH_USES_CFI_TRAPS" in ln or (i > 0 and "CFI_TRAPS" in t.splitlines()[max(0,i-3):i+3]):
                pass
        lines = t.splitlines()
        for i, ln in enumerate(lines):
            if "ARCH_USES_CFI_TRAPS" in ln:
                print("\n".join(lines[max(0, i - 5) : i + 8]))

mh = (k / "include/linux/module.h").read_text()
print("\nVENDOR lines in module.h:")
for i, ln in enumerate(mh.splitlines()):
    if "ANDROID_VENDOR" in ln or "ANDROID_OEM" in ln or "ANDROID_BACKPORT" in ln:
        print(f"{i}: {ln}")

ac = k / "include/generated/autoconf.h"
if ac.exists():
    print("\nautoconf CFI/MODULE related:")
    for ln in ac.read_text().splitlines():
        if any(x in ln for x in ("CFI", "MODULE", "BTF_MODULES", "STATIC_CALL", "KPROBE", "TREE_SRCU", "TRACING", "PRINTK_INDEX", "MITIGATION_ITS")):
            if "CONFIG_" in ln:
                print(ln)
