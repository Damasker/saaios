#!/usr/bin/env python3
"""RE: signed NET/RADIO/L1 SIT that can fire measure/camp/SADR_MEASURE_RSP
without START_NETWORK and without GET_APP in {1,4,5}.

Outputs evidence only — no live send decision here.
"""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
if not DIAG.exists():
    DIAG = Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics")
STREAM = (DIAG / "sit-stream.so").read_bytes()
RIL = (DIAG / "libsitril.so").read_bytes()
MAIN_PATH = DIAG / "fw" / "saaios-probe-b-modem.bin"
IMG = MAIN_PATH.read_bytes()

MAIN, VA0 = 0x16C10, 0x40010000
GET_APP = 0x18EC8C0
START_NET = 0x18E8028  # region; gate @0x18e831a
STATUS_WRAP = 0x14C6626
SADR_CALLER = 0x14B7CA8
L1TUNNEL = 0x1A3E6CC


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def u32(o):
    return struct.unpack_from("<I", IMG, o)[0]


def va(o):
    return VA0 + (o - MAIN)


def off_va(v):
    return MAIN + (v - VA0)


def bl_target(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xF800) != 0xF000 or (hw2 & 0xD000) != 0xD000:
        return None
    s = (hw >> 10) & 1
    imm10 = hw & 0x3FF
    j1, j2, imm11 = (hw2 >> 13) & 1, (hw2 >> 11) & 1, hw2 & 0x7FF
    i1, i2 = ~(j1 ^ s) & 1, ~(j2 ^ s) & 1
    imm32 = (s << 24) | (i1 << 23) | (i2 << 22) | (imm10 << 12) | (imm11 << 1)
    if s:
        imm32 |= ~((1 << 25) - 1) & 0xFFFFFFFF
        if imm32 >= 0x80000000:
            imm32 -= 0x100000000
    return o + 4 + imm32


def movw_imm(o):
    hw, hw2 = u16(o), u16(o + 2)
    if (hw & 0xFBF0) != 0xF240 or (hw2 & 0x8000):
        return None
    i = (hw >> 10) & 1
    imm = (i << 11) | ((hw & 0xF) << 12) | (((hw2 >> 12) & 7) << 8) | (hw2 & 0xFF)
    return imm, (hw2 >> 8) & 0xF


def parse_dynsym(data: bytes) -> dict[str, int]:
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]

    def sh(i: int):
        o = e_shoff + i * e_shentsize
        return {
            "name": struct.unpack_from("<I", data, o)[0],
            "addr": struct.unpack_from("<Q", data, o + 16)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
            "link": struct.unpack_from("<I", data, o + 40)[0],
            "entsize": struct.unpack_from("<Q", data, o + 56)[0],
        }

    names = data[sh(e_shstrndx)["off"] : sh(e_shstrndx)["off"] + sh(e_shstrndx)["size"]]
    syms = {}
    for i in range(e_shnum):
        s = sh(i)
        nm = names[s["name"] :].split(b"\0", 1)[0]
        if nm not in (b".dynsym", b".symtab"):
            continue
        link = sh(s["link"])
        strtab = data[link["off"] : link["off"] + link["size"]]
        ents = s["entsize"] or 24
        for j in range(0, s["size"], ents):
            o = s["off"] + j
            st_name = struct.unpack_from("<I", data, o)[0]
            st_value = struct.unpack_from("<Q", data, o + 8)[0]
            name = strtab[st_name:].split(b"\0", 1)[0].decode("ascii", "replace")
            if name and st_value:
                syms[name] = st_value
    return syms


def decode_builder(data: bytes, addr: int, window: int = 0xC0):
    """Extract first MOVZ Wimm looking like SIT id + nearby length."""
    end = min(len(data), addr + window)
    ids, lens, movs = [], [], []
    o = addr
    while o + 4 <= end:
        w = struct.unpack_from("<I", data, o)[0]
        # MOVZ Wd, #imm16, LSL #0  : 0x5280_xxxx (rough)
        if (w & 0xFFC00000) == 0x52800000:
            imm = (w >> 5) & 0xFFFF
            movs.append(imm)
            if 0x600 <= imm <= 0x9FF or 0x200 <= imm <= 0x2FF or 0x800 <= imm <= 0x9FF:
                ids.append(imm)
            if imm in (12, 13, 14, 16, 24, 32, 48, 64, 71, 72, 86, 246) or (
                8 <= imm <= 0x100 and imm % 2 == 0
            ):
                lens.append(imm)
        o += 4
    return ids, lens, movs[:24]


def index_movw_opcode_like(wanted: set[int], limit_per: int = 8):
    """Single-pass Thumb MOVW index; skip MOVW+MOVT VA pairs."""
    hits = {i: [] for i in wanted}
    end = min(len(IMG) - 4, MAIN + 0x05917ACC)
    for o in range(MAIN, end, 2):
        m = movw_imm(o)
        if not m or m[0] not in wanted:
            continue
        nxt, nxt2 = u16(o + 4), u16(o + 6)
        if (nxt & 0xFBF0) == 0xF2C0 and not (nxt2 & 0x8000):
            if ((nxt2 >> 8) & 0xF) == m[1]:
                continue
        bucket = hits[m[0]]
        if len(bucket) < limit_per:
            bucket.append(o)
    return hits


def neighborhood_get_app(o: int, radius: int = 0x120):
    """Any BL GET_APP / CMP #1/#4/#5 near site."""
    lo, hi = max(MAIN, o - radius), min(len(IMG) - 4, o + radius)
    bls, cmps = [], []
    for p in range(lo, hi, 2):
        t = bl_target(p)
        if t == GET_APP:
            bls.append(hex(va(p)))
        hw = u16(p)
        # CMP Rn, #imm8 Thumb
        if (hw & 0xFF00) == 0x2800:
            imm = hw & 0xFF
            if imm in (1, 2, 4, 5):
                cmps.append((hex(va(p)), imm))
        # CMP.W Rn, #imm
        if p + 4 <= hi:
            hw2 = u16(p + 2)
            if (hw & 0xFBF0) == 0xF1B0:
                # approximate
                pass
    return bls, cmps


def cstr_near(o: int, lim: int = 80):
    # walk back for printable
    s = []
    p = o
    while p > 0 and IMG[p - 1] >= 0x20 and IMG[p - 1] < 0x7F and len(s) < lim:
        p -= 1
    q = p
    while q < len(IMG) and 0x20 <= IMG[q] < 0x7F and len(s) < lim:
        s.append(chr(IMG[q]))
        q += 1
    return "".join(s)


print("=== sit-stream Network/Radio builders (scan/measure/camp-ish) ===")
syms = parse_dynsym(STREAM)
keys = sorted(
    k
    for k in syms
    if any(
        x in k
        for x in (
            "Network",
            "Scan",
            "Measure",
            "Camp",
            "CellInfo",
            "Band",
            "Selection",
            "Operator",
            "Radio",
            "Sadr",
            "SADR",
            "AllowData",
            "Voice",
            "PsAttach",
            "Attach",
            "Detach",
            "Emergency",
            "Manual",
        )
    )
    and "Build" in k
)
interesting = []
for k in keys:
    ids, lens, movs = decode_builder(STREAM, syms[k])
    tag = ""
    if any(x in k.lower() for x in ("scan", "measure", "camp", "sadr", "available", "selection")):
        tag = " ★"
        interesting.append((k, syms[k], ids, lens))
    print(f"  {k} @{hex(syms[k])} ids={ids[:6]} lens={lens[:4]} movs={movs[:8]}{tag}")

print("\n=== ★ scan/measure/camp/selection candidates ===")
for k, a, ids, lens in interesting:
    print(f"  {k} @{hex(a)} ids={ids} lens={lens}")

# Also hunt string names in sit-stream
print("\n=== sit-stream string needles ===")
needles = [
    b"BuildGetAvailableNetworks",
    b"BuildStartNetworkScan",
    b"BuildStopNetworkScan",
    b"BuildSetNetworkSelectionManual",
    b"BuildSetNetworkSelectionAuto",
    b"BuildGetNeighboring",
    b"BuildGetCellInfo",
    b"NetworkScan",
    b"SADR",
    b"Measure",
]
for n in needles:
    i = STREAM.find(n)
    print(f"  {n!r}: stream_off={hex(i) if i>=0 else None}")

print("\n=== libsitril string needles (who calls scan) ===")
for n in [
    b"startNetworkScan",
    b"getAvailableNetworks",
    b"SET_NETWORK_SELECTION",
    b"QUERY_AVAILABLE",
    b"START_NETWORK",
    b"SADR",
]:
    hits = []
    off = 0
    while True:
        i = RIL.find(n, off)
        if i < 0:
            break
        hits.append(hex(i))
        off = i + 1
        if len(hits) >= 5:
            break
    print(f"  {n!r}: {hits}")

# MAIN: SADR_MEASURE_RSP producers already known; check if any SIT opcode
# handler BL STATUS_WRAP or posts measure without START_NETWORK
print("\n=== MAIN: MOVW SIT ids near STATUS_WRAP / SADR / START_NETWORK ===")
cand_ids = [
    0x0700,
    0x0701,
    0x0702,
    0x0703,
    0x0704,
    0x0705,
    0x0706,
    0x0707,
    0x0708,
    0x0709,
    0x070A,
    0x070B,
    0x070C,
    0x070D,
    0x0710,
    0x0711,
    0x0712,
    0x0718,
    0x071A,
    0x0738,
    0x073A,
    0x073E,
    0x0746,
    0x0749,
    0x074D,
    0x0750,
    0x0800,
    0x0801,
    0x0908,
]
print("  indexing MOVW (single pass)...")
movw_hits = index_movw_opcode_like(set(cand_ids), limit_per=6)
for opp in cand_ids:
    sites = movw_hits.get(opp) or []
    if not sites:
        continue
    for o in sites[:3]:
        bls, cmps = neighborhood_get_app(o)
        near_start = abs(o - off_va(START_NET)) < 0x400
        near_sadr = abs(o - SADR_CALLER) < 0x200 or abs(o - L1TUNNEL) < 0x200
        flag = []
        if bls:
            flag.append(f"BL_GET_APP@{bls}")
        if cmps:
            flag.append(f"CMP{cmps}")
        if near_start:
            flag.append("near_START_NET")
        if near_sadr:
            flag.append("near_SADR/L1TUNNEL")
        print(f"  id={hex(opp)} site_va={hex(va(o))} {' '.join(flag) if flag else 'no_GET_APP_in_±0x120'}")

print("\n=== MAIN: who BLs STATUS_WRAP (expect SADR + L1TUNNEL only) ===")
callers = []
for o in range(MAIN, min(len(IMG) - 4, MAIN + 0x05917ACC), 2):
    if bl_target(o) == STATUS_WRAP:
        callers.append(hex(va(o)))
print(f"  callers={callers}")

print("\n=== MAIN: SADR_MEASURE / Not camped / GapMeasure strings ===")
for n in [
    b"SADR_MEASURE_RSP",
    b"SADR_GAP_MEASURE_PAUSE",
    b"Not camped on any frequency",
    b"START_NETWORK Ignored",
    b"SIM is not ready",
    b"measurementModifyReq",
]:
    off = 0
    hits = []
    while True:
        i = IMG.find(n, off)
        if i < 0:
            break
        hits.append(hex(va(i)) if MAIN <= i < MAIN + 0x05917ACC else hex(i))
        off = i + 1
        if len(hits) >= 4:
            break
    print(f"  {n!r}: {hits}")

# Decode specific builders we care about by name from dynsym
print("\n=== Focused builder decode (Available/Scan/Manual/Auto) ===")
focus = [k for k in syms if any(x in k for x in (
    "AvailableNetwork", "NetworkScan", "NetworkSelection", "GetCellInfo",
    "QueryAvailable", "SetNetwork", "GetOperator", "GetSignal", "GetVoice",
    "GetDataRegistration", "SetRadio", "GetRadio", "Emergency", "ManualRat",
))]
for k in sorted(focus):
    ids, lens, movs = decode_builder(STREAM, syms[k], window=0x100)
    print(f"  {k} @{hex(syms[k])} ids={ids[:8]} lens={lens[:6]} movs={movs[:12]}")

print("\n=== Verdict scaffolding ===")
print("STATUS_WRAP callers must stay 2 (SADR_MEASURE_RSP + L1TUNNEL).")
print("If no signed SIT MOVW sites BL STATUS_WRAP or post SADR without camp,")
print("then L1/scan live try is DEAD under bans.")
print("DONE")
