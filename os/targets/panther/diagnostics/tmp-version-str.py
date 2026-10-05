#!/usr/bin/env python3
from pathlib import Path

img = Path("fw/saaios-probe-b-modem.bin").read_bytes()
for n in [b"g5300q-", b"15346003", b"14784800", b"B-15346003"]:
    j = img.find(n)
    print(n, hex(j) if j >= 0 else None)
    if j is not None and j >= 0:
        a = max(0, j - 30)
        b = min(len(img), j + 80)
        chunk = bytes(x if 32 <= x < 127 else 0x2E for x in img[a:b])
        print(" ", chunk.decode("ascii"))

print("--- dispatch head ---")
print(Path("/tmp/present2-dispatch.out").read_text()[:4000])
