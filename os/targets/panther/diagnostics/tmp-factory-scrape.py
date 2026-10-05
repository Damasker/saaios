#!/usr/bin/env python3
import urllib.request, re, json
from pathlib import Path

UA = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"}
out = []
for url in [
    "https://developers.google.com/android/images",
    "https://developers.google.com/android/ota",
]:
    try:
        req = urllib.request.Request(url, headers=UA)
        html = urllib.request.urlopen(req, timeout=40).read().decode("utf-8", "replace")
        out.append(f"URL {url} len={len(html)}")
        abs_links = re.findall(
            r"https://dl\.google\.com/dl/android/aosp/[a-z0-9._-]+\.zip", html
        )
        names = re.findall(
            r"([a-z0-9]+-[a-z0-9.]+-factory-[a-f0-9]+\.zip)", html
        )
        out.append(f"  abs={len(abs_links)} names={len(names)}")
        want = ("panther", "cheetah", "lynx", "felix")
        filt = [n for n in sorted(set(names + [a.rsplit('/',1)[-1] for a in abs_links])) if any(w in n for w in want)]
        out.append(f"  filtered={len(filt)}")
        for n in filt[:60]:
            out.append(f"   {n}")
        i = html.lower().find("panther")
        out.append(f"  panther_idx={i}")
        if i >= 0:
            out.append(repr(html[i : i + mid if (mid:=220) else 220]))
    except Exception as e:
        out.append(f"FAIL {url}: {type(e).__name__}: {e}")

Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/tmp-factory-scrape.out").write_text(
    "\n".join(out)
)
print("wrote", len(out), "lines")
