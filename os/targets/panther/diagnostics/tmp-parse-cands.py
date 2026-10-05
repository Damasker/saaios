#!/usr/bin/env python3
from pathlib import Path

t = Path("verify-p2-rap.out").read_text()
# all has_#636c lines
for line in t.splitlines():
    if "has_#636c" in line or "litpool" in line or line.startswith("MMC_") or "WRAP_A_parent" in line or "parent_wide" in line:
        print(line)

print("\n==== ALL cand headers ====")
for line in t.splitlines():
    if line.startswith("cand "):
        print(line)

print("\n==== litpool section ====")
i = t.find("litpool ptrs")
print(t[i : i + 2000] if i >= 0 else "missing")

print("\n==== WRAP_A_parent ====")
i = t.find("WRAP_A_parent")
print(t[i : i + 1500] if i >= 0 else "missing")

# exhaustive RAP/NV section end
e = Path("exhaustive-fna.out").read_text()
i = e.find("Search RAP")
print("\n==== RAP/NV ====")
print(e[i : i + 2500] if i >= 0 else "missing")
