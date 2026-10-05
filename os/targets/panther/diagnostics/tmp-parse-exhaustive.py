#!/usr/bin/env python3
from pathlib import Path

t = Path("exhaustive-fna.out").read_text()
print(t[:6000])
print("\n==== FN_A body / WRAP ====")
for k in ["FN_A_body", "WRAP_SIM_body", "WRAP_A_body", "DISPATCH_CDMA", "Wide string", "InitRapMap", "RapMap", "MOVS#2", "ALL BL"]:
    i = t.find(k)
    print(f"\n## {k} @ {i}")
    if i >= 0:
        print(t[i : i + 2800])
