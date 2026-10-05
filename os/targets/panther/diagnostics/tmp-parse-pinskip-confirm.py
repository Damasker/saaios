#!/usr/bin/env python3
from pathlib import Path

t = Path("pinskip-confirm.out").read_text()
for k in [
    "PINSKIP_CLUSTER",
    "IS_ESIM_CHECK",
    "invstate",
    "callers into",
    "ALL STRB",
    "SET_APP callers",
    "imm counts",
    "MOVW r2,#0x11ce",
]:
    i = t.find(k)
    print("====", k, "====")
    print(t[i : i + 2500] if i >= 0 else "missing")
    print()
