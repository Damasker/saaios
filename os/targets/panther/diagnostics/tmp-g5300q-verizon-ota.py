#!/usr/bin/env python3
"""HEAD panther Verizon OTAs; download one early Verizon build; scan radio for g5300q CDMA."""
from __future__ import annotations
import hashlib, io, re, struct, urllib.request, zipfile
from pathlib import Path

OUT = Path("/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt")
OUT.mkdir(parents=True, exist_ok=True)
UA = {"User-Agent": "Mozilla/5.0"}
EU = "491993b01530f3ddd6d100bf9f2af7a21934de4bbf7469724ef8e38fc55e5487"


def head(url):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    with urllib.request.urlopen(req, timeout=25) as r:
        return r.status, int(r.headers.get("Content-Length") or 0)


def get(url, timeout=900):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


# From developers.google.cn/android/ota panther section (Verizon-tagged)
# URL pattern: panther-ota-<build>-<sha256[:8]>.zip
CANDS = [
    # Early Android 13 Verizon — highest chance of CDMA RatMap if any
    ("TQ3A.230605.012", "NEED"),  # may not be verizon
    ("TQ3C.230605.010.D1", "NEED"),  # felix verizon — wrong device
]

# Exact panther verizon rows from OTA page dump — parse file
ota_txt = Path(
    "/mnt/c/Users/Admin/.cursor/projects/c-Users-Admin-Projects-saaios-som/"
    "agent-tools/29a3853d-f809-4968-9bb1-22bb11fcb17c.txt"
).read_text(encoding="utf-8", errors="replace")

# Slice panther section
i = ota_txt.find('## "panther" for Pixel 7')
j = ota_txt.find('## "bluejay"', i)
sec = ota_txt[i:j]
rows = re.findall(
    r"\(([A-Z0-9.]+),\s*[^)]*Verizon[^)]*\)\s*\|\s*Link\s*\|\s*([a-f0-9]{64})",
    sec,
)
print(f"panther Verizon rows={len(rows)}")
urls = []
for build, sha in rows[:15]:
    u = f"https://dl.google.com/dl/android/aosp/panther-ota-{build.lower()}-{sha[:8]}.zip"
    urls.append((build, sha, u))
    print(f"  {build} {sha[:8]}...")

ok = []
for build, sha, u in urls:
    try:
        st, cl = head(u)
        print(f"HEAD {build} {st} {cl}")
        if st == 200:
            ok.append((build, sha, u, cl))
    except Exception as e:
        print(f"HEAD fail {build}: {type(e).__name__}: {e}")

print(f"downloadable verizon OTAs={len(ok)}")


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
    print(f"SCAN {label} sha={h[:16]}… g5300q={gq} g5300g={gg} no_cdma={no} enable={en} ver={ver!r}")
    return (gq and no == 0), (h == EU), h


def extract_radio_from_ota(data: bytes):
    """OTA zips contain payload.bin; radio may be in IMAGE or care_map. Try radio.img members first."""
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        names = z.namelist()
        print("  members sample:", names[:20], "count", len(names))
        for n in names:
            low = n.lower()
            if low.endswith("radio.img") or low.endswith("modem.img") or "radio.img" in low:
                return z.read(n), n
        # payload.bin — too heavy to fully decode here; look for embedded g5300q by scanning zip raw
    # Fallback: scan whole zip bytes for version string (radio blob embedded)
    return None, None


# Prefer oldest Verizon OTA (most likely CDMA-era)
ok_sorted = sorted(ok, key=lambda x: x[0])  # build id chronological-ish
# Prefer TQ3* / TD1* / UP1*
prefer = [x for x in ok if x[0].startswith(("TD", "TQ", "UP", "UQ", "AP"))]
pick = (prefer or ok_sorted)[:1]
print("pick", pick)

for build, sha, u, cl in pick:
    if cl > 4_500_000_000:
        print("too big", cl)
        continue
    name = u.rsplit("/", 1)[-1]
    zp = OUT / name
    if not zp.exists():
        print(f"DL {name} ({cl})...")
        zp.write_bytes(get(u))
    data = zp.read_bytes()
    # Verify sha
    h = hashlib.sha256(data).hexdigest()
    print(f"zip sha match={h==sha} got={h[:16]}")
    radio, label = extract_radio_from_ota(data)
    if radio:
        interesting, eu, rh = scan(radio, label)
        if interesting and not eu:
            staged = OUT / f"STAGED-panther-verizon-{build}-{rh[:12]}.img"
            staged.write_bytes(radio)
            print("STAGED", staged)
        else:
            (OUT / f"scan-panther-ota-{build}.txt").write_text(
                f"interesting={interesting} eu={eu} sha={rh}\n"
            )
    else:
        # Scan zip for RatMap strings without extracting payload
        no = data.count(b"No CDMA in SupportedRatMap")
        en = data.count(b"EnableCdmaRat")
        gq = data.count(b"g5300q-")
        gg = data.count(b"g5300g-")
        print(f"zip-scan no_cdma={no} enable={en} g5300q={gq} g5300g={gg}")
        i = data.find(b"g5300q-")
        if i < 0:
            i = data.find(b"g5300g-")
        if i >= 0:
            print("ver", data[i : i + 52].split(b"\x00", 1)[0])
        (OUT / f"scan-panther-ota-{build}.txt").write_text(
            f"no_radio_member no_cdma={no} enable={en} g5300q={gq} g5300g={gg}\n"
        )

print("DONE")
