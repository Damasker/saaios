#!/usr/bin/env python3
"""Panther-section-only Verizon OTA HEAD for g5300q-era candidates."""
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
# Slice each panther section; keep last (most recent page dump)
markers = [m.start() for m in re.finditer(r'##\s+"panther"\s+for Pixel 7', dump)]
print(f"panther_sections={len(markers)}")
if not markers:
    markers = [m.start() for m in re.finditer(r"panther.*Pixel 7", dump, re.I)]
    print(f"alt markers={len(markers)}")

sections = []
for i, start in enumerate(markers):
    end = markers[i + 1] if i + 1 < len(markers) else None
    # next ## "device" heading
    nxt = re.search(r'\n##\s+"[a-z]', dump[start + 10 :])
    if nxt:
        end2 = start + 10 + nxt.start()
        end = min(end, end2) if end else end2
    sections.append(dump[start:end])

sec = sections[-1] if sections else dump
print(f"using section len={len(sec)} preview={sec[:80]!r}")

rows = re.findall(
    r"\(([A-Z0-9.]+),\s*[^)]*Verizon[^)]*\)\s*\|\s*Link\s*\|\s*([a-f0-9]{64})",
    sec,
)
print(f"panther_verizon_rows={len(rows)}")
by: dict[str, str] = {}
for build, sha in rows:
    by[build] = sha
for b in sorted(by):
    print(f"  {b} {by[b][:8]}")

# Prefer: already-known TD1A + any 2025+ (g5300q) + mid
cands = []
for b, sha in by.items():
    cands.append((b, sha))
# Sort: TD1A first, then by build id
cands.sort(key=lambda x: (0 if x[0].startswith("TD1A") else 1, x[0]))

print("\nHEAD all panther Verizon:")
ok = []
for build, sha in cands:
    u = f"https://dl.google.com/dl/android/aosp/panther-ota-{build.lower()}-{sha[:8]}.zip"
    try:
        st, cl = head(u)
        print(f"  {build} {sha[:8]} {st} {cl}")
        if st == 200:
            ok.append((build, sha, u, cl))
    except Exception as e:
        print(f"  {build} {sha[:8]} FAIL {type(e).__name__}")

out = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/"
    "fw/cdma-hunt/verizon-candidates-panther.txt"
)
out.write_text(
    "\n".join(f"{b} {s} {cl} {u}" for b, s, u, cl in ok) + ("\n" if ok else ""),
    encoding="utf-8",
)
print(f"ok={len(ok)} wrote {out}")
