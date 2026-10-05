#!/usr/bin/env python3
"""Parse CN/US OTA page dump for panther Verizon builds incl. 2025+."""
from __future__ import annotations

import re
import urllib.request
from pathlib import Path

DUMPS = [
    Path(
        "/mnt/c/Users/Admin/.cursor/projects/c-Users-Admin-Projects-saaios-som/"
        "agent-tools/baf1f1d7-8f80-4dfc-a9ce-fc047e516895.txt"
    ),
    Path(
        "/mnt/c/Users/Admin/.cursor/projects/c-Users-Admin-Projects-saaios-som/"
        "agent-tools/29a3853d-f809-4968-9bb1-22bb11fcb17c.txt"
    ),
]
UA = {"User-Agent": "Mozilla/5.0"}


def head(url: str):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.status, int(r.headers.get("Content-Length") or 0)


for dump_path in DUMPS:
    if not dump_path.exists():
        print(f"missing {dump_path}")
        continue
    dump = dump_path.read_text(encoding="utf-8", errors="replace")
    print(f"\n=== {dump_path.name} len={len(dump)} ===")
    print(f"panther mentions={len(re.findall('panther', dump, re.I))}")
    # URLs
    urls = sorted(set(re.findall(r"panther-ota-[a-z0-9._-]+\.zip", dump, re.I)))
    print(f"panther-ota urls={len(urls)}")
    for u in urls:
        print(f"  {u}")
    # section slice
    m = re.search(r'##\s+"panther"[^\n]*', dump)
    if not m:
        m = re.search(r"panther.{0,40}Pixel 7", dump, re.I)
    if m:
        start = m.start()
        nxt = re.search(r'\n##\s+"[a-z]', dump[start + 5 :], re.I)
        end = start + 5 + nxt.start() if nxt else start + 30000
        sec = dump[start:end]
        print(f"section len={len(sec)}")
        rows = re.findall(
            r"\(([A-Z0-9.]+),\s*[^)]*Verizon[^)]*\)\s*\|\s*Link\s*\|\s*([a-f0-9]{64})",
            sec,
        )
        print(f"verizon rows={len(rows)}")
        for b, s in rows:
            print(f"  {b} {s[:8]}")
        # also BP/BD/CP era any carrier in panther section
        all_rows = re.findall(
            r"\(([A-Z0-9.]+),\s*[^)]*\)\s*\|\s*Link\s*\|\s*([a-f0-9]{64})",
            sec,
        )
        late = [r for r in all_rows if r[0].startswith(("BP", "BD", "CP", "AP2", "AP3"))]
        print(f"late-ish builds in section={len(late)}")
        for b, s in late[:20]:
            print(f"  LATE {b} {s[:8]}")

# HEAD promising late Verizon if we found BP* with Verizon
print("\n=== HEAD late candidates from CN dump (any panther-ota BP/BD) ===")
cn = DUMPS[0].read_text(encoding="utf-8", errors="replace") if DUMPS[0].exists() else ""
for name in sorted(set(re.findall(r"panther-ota-(bp[a-z0-9._-]+|bd[a-z0-9._-]+|cp[a-z0-9._-]+)\.zip", cn, re.I))):
    u = f"https://dl.google.com/dl/android/aosp/{name}"
    try:
        st, cl = head(u)
        print(f"  {name} {st} {cl}")
    except Exception as e:
        print(f"  {name} FAIL {type(e).__name__}")
