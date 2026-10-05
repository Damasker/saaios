#!/usr/bin/env python3
"""Parse panther Verizon OTA rows from cached developers OTA dump; HEAD candidates."""
from __future__ import annotations

import re
import urllib.request
from pathlib import Path

DUMP = Path(
    "/mnt/c/Users/Admin/.cursor/projects/c-Users-Admin-Projects-saaios-som/"
    "agent-tools/29a3853d-f809-4968-9bb1-22bb11fcb17c.txt"
)
UA = {"User-Agent": "Mozilla/5.0"}


def head(url: str):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.status, int(r.headers.get("Content-Length") or 0)


dump = DUMP.read_text(encoding="utf-8", errors="replace")
rows = re.findall(
    r"\(([A-Z0-9.]+),\s*[^)]*Verizon[^)]*\)\s*\|\s*Link\s*\|\s*([a-f0-9]{64})",
    dump,
)
print(f"verizon_rows={len(rows)}")
by: dict[str, str] = {}
for build, sha in rows:
    by[build] = sha  # keep last occurrence
for build in sorted(by):
    print(f"  {build} {by[build][:8]}")

# Priority: early Verizon (CDMA era) + one mid + one late g5300q-era if present
priority = [
    "TD1A.221105.003",  # already downloading (sha 32ef0dee)
    "TD4A.221205.042.B1",
    "TQ3A.230605.012.A1",
    "UQ1A.240205.002.A1",
    "AP1A.240505.005.A1",
]
# Add any CP*/BP* Verizon if present (g5300q era)
for b in sorted(by):
    if b.startswith(("BP", "CP", "AP2", "AP3", "BP1", "BP2")) and b not in priority:
        priority.append(b)

print("\nHEAD priority:")
ok = []
for build in priority:
    sha = by.get(build)
    if not sha:
        print(f"  MISS {build}")
        continue
    u = f"https://dl.google.com/dl/android/aosp/panther-ota-{build.lower()}-{sha[:8]}.zip"
    try:
        st, cl = head(u)
        print(f"  HEAD {build} {sha[:8]} {st} {cl}")
        if st == 200:
            ok.append((build, sha, u, cl))
    except Exception as e:
        print(f"  HEAD fail {build} {sha[:8]}: {type(e).__name__}: {e}")

out = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/verizon-candidates.txt"
)
lines = [f"{b} {s} {cl} {u}" for b, s, u, cl in ok]
out.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
print(f"wrote {out} n={len(ok)}")
