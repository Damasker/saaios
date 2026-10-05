#!/usr/bin/env python3
import subprocess
from pathlib import Path

KO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/pcie-exynos-gs.ko")
CLANG = Path("/home/mike/local/clang/r487747c/bin")
d = subprocess.check_output(
    [str(CLANG / "llvm-objdump"), "-d", "--no-show-raw-insn", str(KO)],
    text=True, errors="ignore",
).splitlines()
start = next(i for i, ln in enumerate(d) if ln.endswith("<exynos_pcie_rc_set_outbound_atu>:"))
# print until next function
for i in range(start, min(len(d), start + 250)):
    print(d[i])
    if i > start and d[i].endswith(">:") and "outbound" not in d[i]:
        break
