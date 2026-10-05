#!/usr/bin/env python3
from pathlib import Path

lib = Path("libsitril.so").read_bytes()
stream = Path("sit-stream.so").read_bytes()

for label, blob in [("libsitril", lib), ("sit-stream", stream)]:
    print(f"=== {label} ===")
    for n in [
        b"RIL_APPSTATE_READY",
        b"RIL_APPSTATE_PIN",
        b"PIN_STATE_DISABLED",
        b"GetPinState",
        b"CheckAndAutoVerifyPin",
        b"OnGetSimStatusDone",
        b"BuildSimStatus",
    ]:
        print(f"  {n.decode()}: {blob.count(n)}")

# Look for code that treats pin state 3 specially near READY
idx = 0
hits = []
while True:
    j = lib.find(b"DISABLED", idx)
    if j < 0:
        break
    a = max(0, j - 50)
    b = min(len(lib), j + 70)
    s = bytes(x if 32 <= x < 127 else 0x2E for x in lib[a:b]).decode()
    if any(k in s.upper() for k in ("PIN", "READY", "APP", "STATE")):
        hits.append(s)
    idx = j + 1
print("DISABLED contexts:", len(hits))
for h in hits[:20]:
    print(" ", h)

# sit-stream GetPinState / GetAppState adapters - search string
for n in [b"GetAppState", b"GetPinState", b"APPSTATE_READY", b"pin state"]:
    j = stream.find(n)
    print("stream", n, hex(j) if j >= 0 else None)
