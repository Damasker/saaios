#!/usr/bin/env python3
"""Find public factory zip URLs for Pixel s5300; HEAD; stage only non-EU g5300q CDMA."""
from __future__ import annotations
import hashlib, io, re, urllib.request, zipfile
from pathlib import Path

OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt")
OUT.mkdir(parents=True, exist_ok=True)
UA = {"User-Agent": "Mozilla/5.0"}
EU_SHA = "491993b01530f3ddd6d100bf9f2af7a21934de4bbf7469724ef8e38fc55e5487"


def get(url, timeout=60):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def head(url):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=25) as r:
        return r.status, int(r.headers.get("Content-Length") or 0)


# Crowdsourced / mirror indexes that embed full factory URLs as text
INDEXES = [
    "https://raw.githubusercontent.com/android-xtreme/android_vendor_google/master/README.md",
    "https://gitlab.com/android_firmware/google_devices/-/raw/master/README.md",
    # Pixel factory image tracker mirrors
    "https://raw.githubusercontent.com/ZipFile/google-pixel-factory-images/master/README.md",
    "https://api.github.com/repos/ZipFile/google-pixel-factory-images/contents/",
]

html_blobs = []
for u in INDEXES:
    try:
        data = get(u, 30)
        print(f"OK {u} {len(data)}")
        html_blobs.append(data.decode("utf-8", "replace"))
    except Exception as e:
        print(f"fail {u}: {type(e).__name__}: {e}")

# Also try Google's archived sitemap-ish via dl listing is not possible; use
# well-known build board pages scraped by third parties
EXTRA = [
    "https://developers.google.com/android/images-archive",
]
for u in EXTRA:
    try:
        data = get(u, 30)
        print(f"OK {u} {len(data)}")
        html_blobs.append(data.decode("utf-8", "replace"))
    except Exception as e:
        print(f"fail {u}: {type(e).__name__}: {e}")

text = "\n".join(html_blobs)
urls = set(re.findall(r"https://dl\.google\.com/dl/android/aosp/[a-z0-9._-]+-factory-[a-f0-9]+\.zip", text))
# relative
for m in re.findall(r"([a-z0-9]+-[a-z0-9.]+-factory-[a-f0-9]+\.zip)", text):
    urls.add("https://dl.google.com/dl/android/aosp/" + m)

want = ("panther", "cheetah", "lynx", "felix", "tangorpro")
filt = sorted(u for u in urls if any(d in u for d in want))
print(f"found factory urls={len(filt)}")
for u in filt[:80]:
    print(" ", u.rsplit("/", 1)[-1])

# Prefer older / US-carrier-ish builds
priority_keys = ("td1a", "tq1a", "tq2a", "tq3a", "up1a", "ap1a", "ap2a", "bp1a", "bp2a", "cp1a", "tp1a", "sq")
pri = [u for u in filt if any(k in u.lower() for k in priority_keys)]
# de-prioritize known EU twin
pri = [u for u in pri if "cp2a.260705.006" not in u and "td1a.221105.001" not in u]
print(f"priority={len(pri)}")

ok = []
for u in pri[:25]:
    name = u.rsplit("/", 1)[-1]
    try:
        st, cl = head(u)
        print(f"HEAD {name} {st} {cl}")
        if st == 200 and 100_000_000 < cl < 5_000_000_000:
            ok.append((u, cl))
    except Exception as e:
        print(f"HEAD fail {name}: {type(e).__name__}")

print(f"downloadable={len(ok)}")


def scan(data, label):
    h = hashlib.sha256(data).hexdigest()
    no = data.count(b"No CDMA in SupportedRatMap")
    en = data.count(b"EnableCdmaRat")
    gq = b"g5300q-" in data
    gg = b"g5300g-" in data
    ver = b""
    i = data.find(b"g5300q-") if gq else data.find(b"g5300g-")
    if i >= 0:
        ver = data[i : i + 52].split(b"\x00", 1)[0]
    print(f"SCAN {label} sha={h[:16]} g5300q={gq} g5300g={gg} no_cdma={no} enable={en} ver={ver!r}")
    interesting = gq and no == 0
    eu = h == EU_SHA
    return interesting, eu, h


# Download at most 2 new zips this turn
n = 0
for u, cl in ok:
    if n >= 2:
        break
    name = u.rsplit("/", 1)[-1]
    zp = OUT / name
    if zp.exists() and zp.stat().st_size > 1_000_000:
        print(f"have {name}")
        data = zp.read_bytes()
    else:
        print(f"DL {name} ({cl})...")
        try:
            data = get(u, 900)
            zp.write_bytes(data)
        except Exception as e:
            print(f"DL fail: {e}")
            continue
    n += 1
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            names = z.namelist()
            inners = [x for x in names if x.endswith(".zip") and "image" in x.lower()]
            radios = [x for x in names if x.endswith(("radio.img", "modem.img"))]
            payload = None
            label = None
            if radios:
                payload = z.read(radios[0])
                label = radios[0]
            elif inners:
                inner = z.read(inners[0])
                with zipfile.ZipFile(io.BytesIO(inner)) as z2:
                    r2 = [x for x in z2.namelist() if x.endswith(("radio.img", "modem.img"))]
                    if r2:
                        payload = z2.read(r2[0])
                        label = r2[0]
            if not payload:
                print(" no radio")
                continue
            interesting, eu, h = scan(payload, label)
            if interesting and not eu:
                staged = OUT / f"STAGED-g5300q-cdma-{h[:12]}.img"
                staged.write_bytes(payload)
                print(f" STAGED {staged}")
            else:
                print(" not staged (EU twin or no CDMA g5300q)")
    except Exception as e:
        print(f"parse fail: {e}")

# If no index URLs, try a few manually-known hash patterns from prior Verizon OTA notes
MANUAL = [
    # from prior err logs / docs if any exact hashes exist in repo notes
]
print("DONE")
