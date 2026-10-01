#!/usr/bin/env python3
from pathlib import Path

p = Path(
    "fw/cdma-hunt/factory-td1a-images/system_ext.img"
)
data = p.read_bytes()
for key in (b"rild_exynos", b"/vendor/bin/hw/rild", b"oem_ipc", b"SIM_INIT"):
    offs = []
    start = 0
    while True:
        i = data.find(key, start)
        if i < 0:
            break
        offs.append(i)
        start = i + 1
        if len(offs) >= 10:
            break
    print(key, offs)
    for o in offs[:4]:
        ctx = data[max(0, o - 40) : o + 80]
        s = "".join(chr(c) if 32 <= c < 127 else "." for c in ctx)
        print(f"  @{o:#x}: {s}")
