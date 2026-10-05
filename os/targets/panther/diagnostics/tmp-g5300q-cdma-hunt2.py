#!/usr/bin/env python3
"""Broader hunt: signed g5300q with CDMA RatMap (not EU B-15346003 twin).

Download/stage only if clearly different from live EU B-15346003.
"""
from __future__ import annotations

import hashlib
import os
import struct
import urllib.request
from pathlib import Path

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"CDMA in InitRapMap",
    b"TCS_CDMA_SUPPORT",
    b"g5300q-",
    b"g5300g-",
]

LIVE_SHA_PREFIX = "449eeab3"  # live MAIN B
EU_CP2A_MODEM_SHA = "491993b0"  # factory CP2A modem already staged

OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt")
OUT.mkdir(parents=True, exist_ok=True)


def scan_bytes(data: bytes, label: str):
    h = hashlib.sha256(data).hexdigest()
    print(f"\n=== {label} ===")
    print(f"  size={len(data)} sha256={h}")
    for s in (b"g5300q-", b"g5300g-", b"B-15", b"A-15", b"B-14", b"A-14"):
        i = data.find(s)
        if i >= 0:
            frag = data[i : i + 56].split(b"\x00", 1)[0]
            print(f"  ver@{i}: {frag.decode('ascii', 'replace')}")
    for n in NEEDLES:
        print(f"  count[{n.decode('ascii','replace')}]={data.count(n)}")
    no_cdma = data.count(b"No CDMA in SupportedRatMap")
    enable = data.count(b"EnableCdmaRat")
    is_g5300q = b"g5300q-" in data
    is_eu_twin = h.startswith(EU_CP2A_MODEM_SHA) or h.startswith(LIVE_SHA_PREFIX)
    interesting = is_g5300q and no_cdma == 0 and (enable > 0 or b"Found CDMA" in data)
    print(f"  verdict: g5300q={is_g5300q} no_cdma_str={no_cdma} enable={enable} eu_twin={is_eu_twin} INTERESTING={interesting}")
    return interesting, h


def scan_path(p: Path):
    if not p.exists() or not p.is_file():
        print(f"MISSING {p}")
        return False, None
    return scan_bytes(p.read_bytes(), str(p))


print("=== local staged ===")
locals_ = [
    Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-cp2a.260705.006-modem.img"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-cp2a.260705.006-radio.img"),
    Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/factory-td1a.221105.001-radio.img"),
]
for p in locals_:
    scan_path(p)

# Broader filesystem hunt for modem/radio images not yet scanned
print("\n=== walk candidates (depth-limited) ===")
roots = [
    "/mnt/c/Users/Admin/Desktop",
    "/mnt/c/Users/Admin/Downloads",
    "/mnt/c/Users/Admin/Projects",
]
seen = set()
cands = []
for root in roots:
    if not os.path.isdir(root):
        continue
    for dirpath, dirnames, filenames in os.walk(root):
        # prune
        base = os.path.basename(dirpath).lower()
        if base in (".git", "node_modules", "target", "__pycache__", ".cache"):
            dirnames[:] = []
            continue
        depth = dirpath[len(root) :].count(os.sep)
        if depth > 6:
            dirnames[:] = []
            continue
        for fn in filenames:
            low = fn.lower()
            if low in ("modem.img", "radio.img") or low.startswith("radio-") and low.endswith(".img"):
                fp = os.path.join(dirpath, fn)
                if fp not in seen:
                    seen.add(fp)
                    try:
                        sz = os.path.getsize(fp)
                    except OSError:
                        continue
                    if 5_000_000 < sz < 200_000_000:
                        cands.append(fp)

print(f"candidates={len(cands)}")
for c in cands:
    print(" ", c, os.path.getsize(c))

interesting_hits = []
for c in cands:
    # skip already-known huge duplicates by quick head+needle
    data = Path(c).read_bytes()
    ok, h = scan_bytes(data, c)
    if ok:
        interesting_hits.append((c, h))

# Public factory index probes — Pixel s5300 family SKUs (HEAD only first)
# Google OTA/factory naming; only download if HEAD size differs from known EU.
URLS = [
    # Known already: panther CP2A / TD1A — skip re-download; probe other devices
    ("cheetah-cp2a", "https://dl.google.com/dl/android/aosp/cheetah-cp2a.260705.006-factory-*.zip"),  # placeholder pattern
]

print("\n=== HEAD probes for alternate Pixel s5300 factory radios ===")
# Concrete known factory zip URLs from prior hunt logs / common AOSP factory list
HEAD_URLS = [
    "https://dl.google.com/dl/android/aosp/panther-cp2a.260705.006-factory-ed94a24e.zip",
    "https://dl.google.com/dl/android/aosp/cheetah-cp2a.260705.006-factory-1a2b3c4d.zip",  # may 404
]

# Better: scrape factory images page for panther/cheetah/lynx/felix/tangorpro that might ship CDMA
FACTORY_PAGE = "https://developers.google.com/android/images"
try:
    html = urllib.request.urlopen(FACTORY_PAGE, timeout=30).read().decode("utf-8", "replace")
    print(f"factory page bytes={len(html)}")
except Exception as e:
    print(f"factory page fetch fail: {e}")
    html = ""

# Extract candidate zip links for s5300-era devices
import re
links = re.findall(r"https://dl\.google\.com/dl/android/aosp/[a-z0-9]+-[a-z0-9.]+-factory-[a-f0-9]+\.zip", html)
# Also relative
links += ["https://dl.google.com/dl/android/aosp/" + m for m in re.findall(
    r"([a-z0-9]+-[a-z0-9.]+-factory-[a-f0-9]+\.zip)", html
)]
# Dedup keep devices of interest
want = ("panther", "cheetah", "lynx", "felix", "tangorpro", "cloudripper", "komodo", "caiman", "tokay")
filt = []
seen_u = set()
for u in links:
    if u in seen_u:
        continue
    seen_u.add(u)
    low = u.lower()
    if any(d in low for d in want):
        filt.append(u)
print(f"factory links matched={len(filt)}")
for u in filt[:40]:
    print(" ", u)

# Prefer older builds that might still ship US/CDMA radio (TD1A / TQ* / UP*)
priority = [u for u in filt if any(x in u.lower() for x in ("td1a", "tq2a", "tq3a", "up1a", "ap1a", "ap2a", "bp1a", "bp2a", "cp1a", "cp2a"))]
print(f"priority builds={len(priority)}")

# HEAD each priority URL; record Content-Length; only download radio if new
staged = []
for u in priority[:60]:
    try:
        req = urllib.request.Request(u, method="HEAD")
        with urllib.request.urlopen(req, timeout=20) as resp:
            cl = resp.headers.get("Content-Length")
            print(f"HEAD {u.split('/')[-1]} len={cl} status={resp.status}")
    except Exception as e:
        print(f"HEAD fail {u.split('/')[-1]}: {type(e).__name__}: {e}")

print("\n=== INTERESTING local hits ===")
for c, h in interesting_hits:
    print(f"  {c} sha={h}")

print("DONE")
