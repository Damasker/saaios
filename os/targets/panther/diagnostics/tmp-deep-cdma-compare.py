#!/usr/bin/env python3
"""Deep-compare AP1A Verizon modem vs EU CP2A for CDMA/RatMap/TOC UDL fit."""
from __future__ import annotations

import hashlib
import struct
from pathlib import Path

HUNT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt"
)
FW = HUNT.parent / "fw"

AP1A = HUNT / "STAGED-ap1a-verizon-aa0dfc6160fc.img"
EU = FW / "factory-cp2a.260705.006-modem.img"
TD1A = FW / "factory-td1a.221105.001-radio.img"

NEEDLES = [
    b"No CDMA in SupportedRatMap",
    b"EnableCdmaRat",
    b"Found CDMA",
    b"SupportedRatMap",
    b"TCS_CDMA_SUPPORT",
    b"DS_TCS_GV_CDMA_SUPPORT",
    b"CDMA",
    b"Cdma",
    b"cdma",
    b"1xADV",
    b"EVDO",
    b"FN_A",
    b"g5300q-",
    b"g5300g-",
    b"TOC",
    b"MAIN",
    b"NV",
]


def digest(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        while True:
            b = f.read(8 * 1024 * 1024)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def scan(p: Path):
    data = p.read_bytes()
    print(f"\n=== {p.name} size={len(data)} sha={digest(p)} ===")
    print(f"head16={data[:16].hex()} ascii={data[:16]!r}")
    # common headers
    if data[:4] == b"FBPK":
        print("header=FBPK")
    elif data[:4] == b"\x7fELF":
        print("header=ELF")
    else:
        print(f"header_unknown={data[:8]!r}")
    i = data.find(b"g5300q-")
    if i < 0:
        i = data.find(b"g5300g-")
    if i >= 0:
        print(f"ver@{i}={data[i:i+64].split(bchr(0) if False else bytes([0]),1)[0]!r}")
        # fix: use null split properly
        print(f"ver={data[i:i+64].split(b'\\x00' if False else b'\\0', 1)}")
    # redo ver cleanly
    for key in (b"g5300q-", b"g5300g-"):
        j = 0
        n = 0
        while n < 5:
            k = data.find(key, j)
            if k < 0:
                break
            ver = data[k : k + 64].split(b"\0", 1)[0]
            print(f"  {key.decode()}@{k}: {ver!r}")
            j = k + 1
            n += 1
    print("needle counts:")
    for n in NEEDLES:
        c = data.count(n)
        if c:
            print(f"  {n!r}: {c}")
    # TOC-ish markers used by CPIF UDL
    for marker in (b"TOC\x00", b"BIN\x00", b"MAIN", b"NV\x00\x00", b"CP_INFO"):
        print(f"  marker {marker!r}: {data.count(marker)}")
    return data


# Fix the botched ver print by clean function
def scan2(p: Path):
    data = p.read_bytes()
    print(f"\n=== {p.name} size={len(data)} sha={digest(p)} ===")
    print(f"head16={data[:16].hex()}")
    if data[:4] == b"FBPK":
        print("header=FBPK (likely g5300g-era packaging)")
    elif data[0:4] == b"\x7fELF":
        print("header=ELF")
    else:
        # Shannon TOC often starts with size/count fields
        print(f"header_raw={data[:32]!r}")
    for key in (b"g5300q-", b"g5300g-"):
        j = 0
        for _ in range(6):
            k = data.find(key, j)
            if k < 0:
                break
            ver = data[k : k + 64].split(b"\0", 1)[0]
            print(f"  ver {ver!r} @{k}")
            j = k + 1
    print("needles:")
    for n in NEEDLES:
        c = data.count(n)
        if c:
            print(f"  {n!r}: {c}")
    # Sample unique CDMA-ish strings around CDMA
    idx = 0
    shown = 0
    while shown < 15:
        k = data.find(b"CDMA", idx)
        if k < 0:
            break
        ctx = data[max(0, k - 20) : k + 40]
        # printable-ish
        s = "".join(chr(b) if 32 <= b < 127 else "." for b in ctx)
        print(f"  CDMA@{k}: {s}")
        idx = k + 4
        shown += 1
    return data


for p in (AP1A, EU, TD1A):
    if p.exists():
        scan2(p)
    else:
        print("missing", p)

# UDL gate decision
print("\n=== UDL GATE ===")
ap = AP1A.read_bytes()
eu = EU.read_bytes()
ap_no = ap.count(b"No CDMA in SupportedRatMap")
eu_no = eu.count(b"No CDMA in SupportedRatMap")
ap_en = ap.count(b"EnableCdmaRat")
ap_gq = ap.count(b"g5300q-")
print(f"AP1A: g5300q={ap_gq} no_cdma={ap_no} enable={ap_en} size={len(ap)}")
print(f"EU:   no_cdma={eu_no} size={len(eu)}")
# Heuristic: usable if g5300q and (no_cdma==0) and size>~50MB and header not FBPK
usable = (
    ap_gq > 0
    and ap_no == 0
    and len(ap) > 50_000_000
    and ap[:4] != b"FBPK"
)
print(f"heuristic_usable_signed_g5300q_no_eu_string={usable}")
print(
    "NOTE: EnableCdmaRat/Found CDMA both 0 — CDMA RatMap positive evidence weak; "
    "absence of EU No-CDMA string is necessary but not sufficient."
)
# Compare CDMA string density
print(f"AP1A CDMA count={ap.count(b'CDMA')} EU CDMA count={eu.count(b'CDMA')}")
print(f"AP1A Cdma count={ap.count(b'Cdma')} EU Cdma count={eu.count(b'Cdma')}")
