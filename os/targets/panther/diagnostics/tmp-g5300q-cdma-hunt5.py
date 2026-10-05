#!/usr/bin/env python3
"""HEAD/scan CalyxOS-known factory URLs for g5300q CDMA RatMap."""
from __future__ import annotations
import hashlib, io, re, urllib.request, zipfile
from pathlib import Path

OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt")
OUT.mkdir(parents=True, exist_ok=True)
UA = {"User-Agent": "Mozilla/5.0"}
EU_MODEM = "491993b01530f3ddd6d100bf9f2af7a21934de4bbf7469724ef8e38fc55e5487"


def get(url, timeout=60):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def head(url):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=25) as r:
        return r.status, int(r.headers.get("Content-Length") or 0)


# CalyxOS pixel device vars (raw from gitlab/github mirrors)
VAR_URLS = [
    "https://gitlab.com/CalyxOS/scripts/-/raw/master/vars/panther",
    "https://gitlab.com/CalyxOS/scripts/-/raw/master/vars/cheetah",
    "https://gitlab.com/CalyxOS/scripts/-/raw/master/vars/lynx",
    "https://gitlab.com/CalyxOS/scripts/-/raw/master/vars/felix",
    "https://raw.githubusercontent.com/CalyxOS/scripts/master/vars/panther",
    "https://raw.githubusercontent.com/CalyxOS/scripts/master/vars/cheetah",
    "https://raw.githubusercontent.com/CalyxOS/scripts/master/vars/lynx",
    "https://raw.githubusercontent.com/CalyxOS/scripts/master/vars/felix",
]

urls = set()
for u in VAR_URLS:
    try:
        t = get(u, 30).decode("utf-8", "replace")
        print(f"OK {u} ({len(t)})")
        for m in re.findall(r"https://dl\.google\.com/dl/android/aosp/[a-z0-9._-]+\.zip", t):
            urls.add(m)
            print(" ", m.rsplit("/", 1)[-1])
    except Exception as e:
        print(f"fail {u}: {type(e).__name__}: {e}")

# Known-good from web search this turn
urls.add("https://dl.google.com/dl/android/aosp/cheetah-cp2a.260705.006-factory-23d564ad.zip")
# Prior panther
urls.add("https://dl.google.com/dl/android/aosp/panther-cp2a.260705.006-factory-ed94a24e.zip")
urls.add("https://dl.google.com/dl/android/aosp/panther-td1a.221105.001-factory-10a338fe.zip")

print(f"\nunique urls={len(urls)}")
ok = []
for u in sorted(urls):
    name = u.rsplit("/", 1)[-1]
    try:
        st, cl = head(u)
        print(f"HEAD {name} {st} {cl}")
        if st == 200:
            ok.append((u, cl))
    except Exception as e:
        print(f"HEAD fail {name}: {type(e).__name__}")


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
    print(f"SCAN {label} sha={h} g5300q={gq} g5300g={gg} no_cdma={no} enable={en} ver={ver!r}")
    return gq and no == 0, h == EU_MODEM or h.startswith("491993b0"), h


# Prefer cheetah CP2A (correct hash now) — radio-only extract; stage only if not EU twin
targets = [u for u, cl in ok if "cheetah-cp2a.260705.006-factory-23d564ad" in u]
# Also any non-cp2a / non-td1a.221105.001 if present
targets += [u for u, cl in ok if "factory" in u and "panther-cp2a" not in u and "td1a.221105.001" not in u and u not in targets]
print(f"\nextract targets={targets[:3]}")

for u in targets[:1]:  # only cheetah this turn (likely EU; confirm)
    name = u.rsplit("/", 1)[-1]
    zp = OUT / name
    cl = dict(ok)[u]
    if zp.exists() and zp.stat().st_size > 1_000_000_000:
        print(f"have {name}")
        data = zp.read_bytes()
    else:
        print(f"DL {name} ({cl} bytes) — may take a while...")
        data = get(u, 1200)
        zp.write_bytes(data)
        print(f"wrote {len(data)}")
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        names = z.namelist()
        inners = [x for x in names if x.endswith(".zip") and "image" in x.lower()]
        payload = None
        label = None
        if inners:
            inner = z.read(inners[0])
            with zipfile.ZipFile(io.BytesIO(inner)) as z2:
                r2 = [x for x in z2.namelist() if x.endswith(("radio.img", "modem.img"))]
                print("inner radios", r2[:6])
                if r2:
                    payload = z2.read(r2[0])
                    label = r2[0]
        if payload:
            interesting, eu, h = scan(payload, f"cheetah/{label}")
            out = OUT / f"cheetah-cp2a-radio-{h[:12]}.img"
            # Keep scan artifact small note; only STAGED if interesting
            if interesting and not eu:
                out.write_bytes(payload)
                print(f"STAGED {out}")
            else:
                print(f"not staged (interesting={interesting} eu_twin={eu}); leave zip for audit")
                # write tiny sidecar report
                (OUT / "cheetah-cp2a-scan.txt").write_text(
                    f"sha={h}\ninteresting={interesting}\neu_twin={eu}\n"
                )
        else:
            print("no radio found")

print("DONE")
