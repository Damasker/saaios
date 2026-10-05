#!/usr/bin/env python3
import subprocess
from pathlib import Path

KO = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/pcie-exynos-gs.ko")
CLANG = Path("/home/mike/local/clang/r487747c/bin")

nm = subprocess.check_output([str(CLANG / "llvm-nm"), str(KO)], text=True, errors="ignore")
for ln in nm.splitlines():
    if "outbound" in ln.lower() or "14200000" in ln or "atu" in ln.lower():
        print("NM", ln)

print("--- disasm ---")
d = subprocess.check_output(
    [str(CLANG / "llvm-objdump"), "-d", "--no-show-raw-insn", str(KO)],
    text=True,
    errors="ignore",
)
lines = d.splitlines()
start = None
for i, ln in enumerate(lines):
    if "exynos_pcie_rc_set_outbound_atu" in ln and ln.endswith(">:"):
        start = i
        break
if start is None:
    # try without demangle
    for i, ln in enumerate(lines):
        if "set_outbound_atu" in ln and ">" in ln:
            print("CAND", ln)
            start = i
            break
if start is not None:
    for ln in lines[start : start + 150]:
        print(ln)
        if ln.strip() == "" and "ret" in lines[max(0, lines.index(ln) - 1) : lines.index(ln) + 1]:
            pass
else:
    print("symbol not found")
    for ln in lines:
        if "outbound" in ln.lower():
            print(ln)

# Also search immediates related to bases
print("--- imm hunt ---")
for ln in lines:
    if "0x14200000" in ln or "0x40200000" in ln or "0x40000000" in ln:
        print(ln)
