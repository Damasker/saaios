#!/usr/bin/env python3
from pathlib import Path

p = Path("/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md")
sec = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/tmp-nonstr-runtime-section.md"
).read_text(encoding="utf-8")
t = p.read_text(encoding="utf-8")
marker = "## 2026-10-01: non-string dispatcher / catalog / nanopb RE (no send)"
idx = t.find(marker)
assert idx >= 0, "marker missing"
pre = t[:idx].rstrip() + "\n\n"
p.write_text(pre + sec.rstrip() + "\n", encoding="utf-8")
print("runtime ok", len(pre), len(sec))

bp = Path("/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-BLOCKER.md")
bt = bp.read_text(encoding="utf-8")
old = (
    "Catalog `SIM_INIT` `@0x6de740` | body=2 msgid=`0x2f50` meta=`0x10104` "
    "rsp=0 **`+0x18=4`** (unique; not wire proof)"
)
new = (
    "Catalog `SIM_INIT` `@0x6de740` | body=2 msgid=`0x2f50` meta=`0x10104` "
    "rsp=0 **`+0x18=4`** (INIT-family class tag; not wire token)"
)
if old in bt:
    bp.write_text(bt.replace(old, new), encoding="utf-8")
    print("blocker line fixed")
else:
    print("blocker line not found; skip")
