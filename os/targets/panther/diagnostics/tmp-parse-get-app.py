#!/usr/bin/env python3
from pathlib import Path

t = Path("get-app.out").read_text()
for line in t.splitlines():
    if line.strip().startswith("BL @") or line.startswith("n="):
        print(line)
print("--- GET_APP_STATE ---")
i = t.find("GET_APP_STATE")
print(t[i : i + 800] if i >= 0 else "missing")
for c in [
    "0x1d67a34",
    "0x18fe3c6",
    "0x18ec4f2",
    "0x18e831a",
    "0x18c64b8",
    "0x1a3327c",
    "0x1941440",
    "0x13c0490",
    "0x1433c84",
]:
    i = t.find(f"caller_get@{c}")
    print(f"\n## {c} found={i>=0}")
    if i >= 0:
        print(t[i : i + 1000])
