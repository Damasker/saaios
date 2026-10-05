#!/usr/bin/env python3
from pathlib import Path

t = Path("mminit-disasm.out").read_text()
print(t[:7000])
print("\n==== callers / APIs ====")
i = t.find("callers of")
print(t[i : i + 4000] if i >= 0 else "no callers")
i = t.find("RRM / band")
print(t[i : i + 2000] if i >= 0 else "no rrm")
i = t.find("TCS_Get")
print(t[i : i + 1500] if i >= 0 else "no tcs")
