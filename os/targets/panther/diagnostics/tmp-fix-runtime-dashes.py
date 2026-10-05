#!/usr/bin/env python3
from pathlib import Path

p = Path("/mnt/c/Users/Admin/Projects/saaios-som/docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md")
if not p.exists():
    p = Path(r"C:\Users\Admin\Projects\saaios-som\docs\os\targets\panther\MODEM-RUNTIME-2026-09-24.md")
t = p.read_text(encoding="utf-8")
start = t.find("## 2026-10-01: OEM IPC preprocess RX")
assert start >= 0
end = t.find("\n## ", start + 5)
if end < 0:
    end = len(t)
head, mid, tail = t[:start], t[start:end], t[end:]
repls = [
    (
        "### DBT attribution (preprocess / msgid path ? not litpool-alone)",
        "### DBT attribution (preprocess / msgid path -- not litpool-alone)",
    ),
    ("| `0x95` / ? |", "| `0x95` / ... |"),
    (
        "**gmetrics** clients only ?\n**not** OEM catalog preprocess.",
        "**gmetrics** clients only --\n**not** OEM catalog preprocess.",
    ),
    (
        "`u16 rsp`, ?\n`SIM_INIT` uniquely has `+0x18=4` in this bank (meaning still **unknown** ?\nnot proven as wire token length).",
        "`u16 rsp`, ...\n`SIM_INIT` uniquely has `+0x18=4` in this bank (meaning still **unknown** --\nnot proven as wire token length).",
    ),
    ("builders ? **not** proven", "builders -- **not** proven"),
    (
        "ch=`0x81` ? no `ATTR_NO_LINK_HEADER` ? EXYNOS 12B",
        "ch=`0x81` -- no `ATTR_NO_LINK_HEADER` -- EXYNOS 12B",
    ),
    ("hint only ? **not** wire proof", "hint only -- **not** wire proof"),
]
for a, b in repls:
    if a not in mid:
        print("MISSING:", repr(a[:80]))
    else:
        mid = mid.replace(a, b)
        print("OK:", a[:50])
p.write_text(head + mid + tail, encoding="utf-8")
print("wrote", p)
