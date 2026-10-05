#!/usr/bin/env python3
from pathlib import Path

for name in ["exhaustive-fna.out", "verify-p2-rap.out"]:
    t = Path(name).read_text()
    print(f"\n######## {name} len={len(t)} ########")
    for k in [
        "Wide string",
        "InitRapMap",
        "RapMap",
        "cand ",
        "has_#636c",
        "WRAP_A_parent",
        "litpool",
        "MOVW/MOVT",
        "QM_MM_INIT",
        "stream",
        "CDMA_MEAS",
        "parent_wide",
    ]:
        i = t.find(k)
        if i >= 0:
            print(f"\n==== {k} ====")
            print(t[i : i + 2200])
