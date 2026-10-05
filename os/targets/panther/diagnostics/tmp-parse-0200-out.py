#!/usr/bin/env python3
from pathlib import Path
t = Path("/tmp/sit-0200-tx.out").read_text(errors="replace")
print("lines", t.count("\n"), "BF4", t.count("BF4"), "LDRB", t.count("LDRB"))
for key in ["LDRB.W", "BF4", "sitSend", "SIT_GET", "No CDMA", "DecodeSim", "ALL LDRB"]:
    i = t.find(key)
    print(f"{key}: pos={i}")
# print section from ALL LDRB if present
i = t.find("ALL LDRB")
if i >= 0:
    print(t[i : i + 4000])
else:
    # print middle
    lines = t.splitlines()
    for i, line in enumerate(lines):
        if "BF4" in line or "sitSend" in line or "SIT_GET" in line or "No CDMA" in line:
            print(i, line)
