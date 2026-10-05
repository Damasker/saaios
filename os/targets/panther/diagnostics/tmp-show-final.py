#!/usr/bin/env python3
from pathlib import Path

for name in ["fna-final.out", "latch-rap.out"]:
    t = Path(name).read_text()
    print(f"\n######## {name} ########")
    print(t[:5000])
    if len(t) > 5000:
        print("...\n====TAIL====\n")
        print(t[-3000:])
