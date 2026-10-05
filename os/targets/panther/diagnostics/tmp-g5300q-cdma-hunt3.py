#!/usr/bin/env python3
"""HEAD/download hunt for non-EU g5300q CDMA RatMap among Pixel factory zips."""
from __future__ import annotations
import hashlib, re, urllib.request, zipfile, io, os
from pathlib import Path

OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt")
OUT.mkdir(parents=True, exist_ok=True)
EU_MODEM_SHA = "491993b01530f3ddd6d100bf9f2af7a21934de4bbf7469724ef8e38fc55e5487"

UA = {"User-Agent": "Mozilla/5.0 (compatible; saaios-modem-research/1.0)"}

def fetch(url, timeout=60):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()

def head(url):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=25) as r:
        return r.status, r.headers.get("Content-Length"), r.url

# Factory images index (HTML may embed full URLs)
pages = [
    "https://developers.google.com/android/images",
    "https://developers.google.com/android/ota",
]
html = b""
for p in pages:
    try:
        html += fetch(p, 40)
        print(f"fetched {p} +{len(html)}")
    except Exception as e:
        print(f"fail {p}: {e}")

text = html.decode("utf-8", "replace")
# Multiple patterns Google has used
pats = [
    r"https://dl\.google\.com/dl/android/aosp/[a-z0-9._-]+-factory-[a-f0-9]+\.zip",
    r"https://dl\.google\.com/dl/android/aosp/[a-z0-9._-]+-ota-[a-f0-9]+\.zip",
    r"/dl/android/aosp/([a-z0-9._-]+-factory-[a-f0-9]+\.zip)",
    r"/dl/android/aosp/([a-z0-9._-]+-ota-[a-f0-9]+\.zip)",
]
urls = set()
for pat in pats:
    for m in re.findall(pat, text):
        if m.startswith("http"):
            urls.add(m)
        else:
            urls.add("https://dl.google.com/dl/android/aosp/" + m)

# Device filter: Tensor/s5300-ish Pixels
want = ("panther", "cheetah", "lynx", "felix", "tangorpro", "cloudripper")
filt = sorted(u for u in urls if any(d in u for d in want))
print(f"total urls={len(urls)} filtered={len(filt)}")
for u in filt[:80]:
    print(" ", u)

# Also try known concrete factory ZIP names from prior docs / common builds
# (Google factory URL is deterministic: <device>-<build>-factory-<hash>.zip but hash unknown)
# Use developers page build ids if present
build_ids = sorted(set(re.findall(r"\b([A-Z]{2,3}\d[A-Z]\.\d{6}\.\d{3})\b", text)))
print(f"build_ids sample={build_ids[:30]}")

# HEAD filtered URLs; for factory zips, note size. Download ONLY if we can cheaply
# extract radio without full zip — actually need full download for radio.img.
# Policy: only download if HEAD ok AND device/build not already proven EU-identical.
# Skip panther-cp2a.260705.006 (known EU). Prefer older Verizon-ish / TD* / TQ* /
# and US SKU devices (cheetah/lynx same chip often share radio).

already = {
    "panther-cp2a.260705.006",
    "panther-td1a.221105.001",
}

candidates = []
for u in filt:
    name = u.rsplit("/", 1)[-1]
    skip = any(a in name for a in already)
    # Prefer factory over ota for radio.img
    if "-factory-" not in name:
        continue
    if skip:
        print(f"SKIP known {name}")
        continue
    candidates.append(u)

print(f"HEAD candidates={len(candidates)}")
ok_heads = []
for u in candidates[:40]:
    name = u.rsplit("/", 1)[-1]
    try:
        st, cl, final = head(u)
        print(f"HEAD {name} status={st} len={cl}")
        if st == 200 and cl:
            ok_heads.append((u, int(cl)))
    except Exception as e:
        print(f"HEAD fail {name}: {type(e).__name__}")

# If no HTML links worked, try a curated list of publicly documented factory URLs
CURATED = [
    # From prior hunt logs / common mirrors — may 404
    "https://dl.google.com/dl/android/aosp/cheetah-cp2a.260705.006-factory-ed94a24e.zip",
    "https://dl.google.com/dl/android/aosp/lynx-cp2a.260705.006-factory-ed94a24e.zip",
    "https://dl.google.com/dl/android/aosp/felix-cp2a.260705.006-factory-ed94a24e.zip",
    "https://dl.google.com/dl/android/aosp/panther-td1a.221105.001-factory-10a338fe.zip",
    "https://dl.google.com/dl/android/aosp/cheetah-td1a.221105.001-factory-10a338fe.zip",
    "https://dl.google.com/dl/android/aosp/panther-tq2a.230305.008.e1-factory-2ce2c84c.zip",
    "https://dl.google.com/dl/android/aosp/panther-tq3a.230605.012-factory-c3b91d2a.zip",
    "https://dl.google.com/dl/android/aosp/panther-up1a.231105.001-factory-70a471f3.zip",
    "https://dl.google.com/dl/android/aosp/panther-ap1a.240405.002-factory-f9f9f9f9.zip",
]
print("\n=== curated HEAD ===")
for u in CURATED:
    name = u.rsplit("/", 1)[-1]
    try:
        st, cl, final = head(u)
        print(f"HEAD {name} status={st} len={cl}")
        if st == 200 and cl:
            ok_heads.append((u, int(cl)))
    except Exception as e:
        print(f"HEAD fail {name}: {type(e).__name__}: {e}")

# Dedup ok_heads
seen = set()
uniq = []
for u, cl in ok_heads:
    if u in seen:
        continue
    seen.add(u)
    uniq.append((u, cl))

print(f"\n=== download+scan radio for {len(uniq)} zips (only if not huge duplicates) ===")

def scan_radio_bytes(data, label):
    h = hashlib.sha256(data).hexdigest()
    no = data.count(b"No CDMA in SupportedRatMap")
    en = data.count(b"EnableCdmaRat")
    gq = b"g5300q-" in data
    gg = b"g5300g-" in data
    ver = b""
    i = data.find(b"g5300q-")
    if i < 0:
        i = data.find(b"g5300g-")
    if i >= 0:
        ver = data[i : i + 48].split(b"\x00", 1)[0]
    print(f"  {label} sha={h[:16]}… g5300q={gq} g5300g={gg} no_cdma={no} enable={en} ver={ver!r}")
    interesting = gq and no == 0
    eu_twin = h == EU_MODEM_SHA or h.startswith("491993b0")
    return interesting, eu_twin, h

# Only download first few successful HEADs that look new (not panther-cp2a)
downloaded = 0
for u, cl in uniq:
    name = u.rsplit("/", 1)[-1]
    if "panther-cp2a" in name or "panther-td1a.221105.001" in name:
        print(f"skip re-dl {name}")
        continue
    if cl > 6_000_000_000:
        print(f"skip huge {name} {cl}")
        continue
    # Prefer smaller / older
    out_zip = OUT / name
    if not out_zip.exists():
        print(f"DOWNLOAD {name} ({cl} bytes)…")
        try:
            data = fetch(u, timeout=600)
            out_zip.write_bytes(data)
            print(f"  wrote {out_zip} {len(data)}")
        except Exception as e:
            print(f"  DL fail: {e}")
            continue
    else:
        print(f"have {out_zip}")
        data = out_zip.read_bytes()
    downloaded += 1
    # Extract radio*.img / modem.img from nested zips
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            names = z.namelist()
            print(f"  zip entries={len(names)}")
            inners = [n for n in names if n.endswith(".zip") and "image" in n.lower()]
            radios = [n for n in names if n.endswith("radio.img") or n.endswith("modem.img") or "/radio.img" in n or n.endswith("radio-panther.img")]
            payload = None
            label = None
            if radios:
                with z.open(radios[0]) as f:
                    payload = f.read()
                label = radios[0]
            elif inners:
                with z.open(inners[0]) as f:
                    inner = f.read()
                with zipfile.ZipFile(io.BytesIO(inner)) as z2:
                    n2 = z2.namelist()
                    r2 = [n for n in n2 if n.endswith("radio.img") or n.endswith("modem.img")]
                    print(f"  inner radios={r2[:5]}")
                    if r2:
                        with z2.open(r2[0]) as f:
                            payload = f.read()
                        label = r2[0]
            if payload:
                # stage only if interesting OR clearly different family
                interesting, eu_twin, h = scan_radio_bytes(payload, label)
                if interesting and not eu_twin:
                    staged = OUT / f"STAGED-{name}-{h[:12]}-radio.bin"
                    staged.write_bytes(payload)
                    print(f"  STAGED {staged}")
                elif eu_twin:
                    print("  EU twin — not staged")
                else:
                    print("  not CDMA-g5300q — not staged")
            else:
                print("  no radio/modem in zip")
    except Exception as e:
        print(f"  zip parse fail: {e}")
    if downloaded >= 5:
        print("stop after 5 downloads this turn")
        break

print("DONE")
