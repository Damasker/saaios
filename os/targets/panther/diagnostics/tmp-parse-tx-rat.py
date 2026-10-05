#!/usr/bin/env python3
from pathlib import Path

t = Path("tx-pinskip-rat.out").read_text()
for key in [
    "TX_CAND_18e80",
    "TX_CAND_18c64",
    "NOTIFY_20e184e",
    "real_11d6",
    "real_11ce",
    "NS_SIM_PIN",
    "NoCDMA",
    "SupportedRat",
    "MOVW #0x11d6",
    "MOVW #0x11ce",
    "BL",
]:
    print(key, t.find(key))

i = t.find("NOTIFY_20e184e")
print("\n==== NOTIFY ====")
print(t[i : i + 2500])

i = t.find("=== MOVW #0x11d6")
print("\n==== 11d6 ====")
print(t[i : i + 3500])

i = t.find("=== MOVW #0x11ce")
print("\n==== 11ce ====")
print(t[i : i + 3500])

i = t.find("NoCDMA InitRapMap")
print("\n==== Rat ====")
print(t[i : i + 2500])

i = t.find("0x18e8348")
print("\n==== around 18e8348 ====")
print(t[max(0, i - 800) : i + 2000])

i = t.find("0x18c64b8")
print("\n==== around 18c64b8 ====")
print(t[max(0, i - 200) : i + 2500])

# STRB after app_state in 18e83
i = t.find("TX_CAND_18e80")
chunk = t[i : i + 12000]
for line in chunk.splitlines():
    if "STRB" in line or "0x347" in line or "MOVW r" in line and "0x347" in line:
        print("STR/347:", line)
