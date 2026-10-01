#!/usr/bin/env python3
"""RE: SIT 0x020a SetFacilityLock — pin1 DISABLED path.

Question: is there a safe documented payload (no secrets logging) that can
move pin1/app_state via 0x020a while soft-lock is PIN+pin1=2?

Prior live: SET enable (mode=1) + candidate A + class7 → error_raw=2; pin1 stayed 2.
"""
from __future__ import annotations

import struct
from pathlib import Path

DIAG = Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics")
if not DIAG.exists():
    DIAG = Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics")
STREAM = (DIAG / "sit-stream.so").read_bytes()
RIL = (DIAG / "libsitril.so").read_bytes()
IMG = (DIAG / "fw" / "saaios-probe-b-modem.bin").read_bytes()

MAIN, VA0 = 0x16C10, 0x40010000
GET_APP, SET_APP = 0x18EC8C0, 0x19916D2


def u16(o):
    return struct.unpack_from("<H", IMG, o)[0]


def va(o):
    return VA0 + (o - MAIN)


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


def dump_builder(name: str, addr: int, n: int = 0x100):
    print(f"\n=== {name} @{hex(addr)} ===")
    end = min(len(STREAM), addr + n)
    o = addr
    while o + 4 <= end:
        w = struct.unpack_from("<I", STREAM, o)[0]
        # MOVZ Wd,#imm
        if (w & 0xFFC00000) == 0x52800000:
            imm = (w >> 5) & 0xFFFF
            rd = w & 0x1F
            print(f"  {hex(o)}: MOVZ w{rd},#{hex(imm)} ({imm})")
        # STRB Wt,[Xn,#imm]
        if (w & 0xFFC00000) == 0x39000000:
            imm = (w >> 10) & 0xFFF
            rt = w & 0x1F
            rn = (w >> 5) & 0x1F
            print(f"  {hex(o)}: STRB w{rt},[x{rn},#{imm}]")
        o += 4


syms = parse_dynsym(STREAM)
for name in [
    "BuildSimSetFacilityLock",
    "BuildSimGetFacilityLock",
    "_ZN18ProtocolSimBuilder23BuildSimSetFacilityLockEPciS0_iS0_",
    "_ZN18ProtocolSimBuilder23BuildSimGetFacilityLockEPciS0_iS0_",
]:
    # try mangled via substring
    pass

set_syms = [k for k in syms if "SetFacilityLock" in k or "GetFacilityLock" in k]
print("=== FacilityLock symbols ===")
for k in sorted(set_syms):
    print(f"  {k} @{hex(syms[k])}")
    dump_builder(k, syms[k], 0x140)

# libsitril: setIccLockEnabled / queryFacilityLock
print("\n=== libsitril facility strings ===")
for n in [
    b"setIccLockEnabled",
    b"queryFacilityLock",
    b"SetFacilityLock",
    b"GetFacilityLock",
    b"FACILITY",
    b"SC",
]:
    off = 0
    hits = []
    while True:
        i = RIL.find(n, off)
        if i < 0:
            break
        # show context ascii
        ctx = RIL[i : i + 60].split(b"\0", 1)[0]
        hits.append((hex(i), ctx))
        off = i + 1
        if len(hits) >= 6:
            break
    print(f"  {n!r}: {hits[:4]}")

# MAIN CP: MOVW #0x20a sites and nearby GET_APP / SET_APP / pin1
print("\n=== MAIN MOVW #0x020a sites (±GET_APP/SET_APP/pin) ===")
sites = []
for o in range(MAIN, min(len(IMG) - 4, MAIN + 0x05917ACC), 2):
    m = movw_imm(o)
    if not m or m[0] != 0x020A:
        continue
    nxt = u16(o + 4)
    nxt2 = u16(o + 6)
    if (nxt & 0xFBF0) == 0xF2C0 and not (nxt2 & 0x8000):
        if ((nxt2 >> 8) & 0xF) == m[1]:
            continue  # MOVT pair = VA
    sites.append(o)

print(f"  opcode-like sites: {len(sites)}")
for o in sites[:12]:
    bls = []
    for p in range(max(MAIN, o - 0x180), min(len(IMG) - 4, o + 0x180), 2):
        t = bl_target(p)
        if t in (GET_APP, SET_APP):
            bls.append((hex(va(p)), "GET_APP" if t == GET_APP else "SET_APP"))
    # pin1-ish CMP #1/#2/#3 near
    cmps = []
    for p in range(max(MAIN, o - 0x100), min(len(IMG) - 2, o + 0x100), 2):
        hw = u16(p)
        if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) in (0, 1, 2, 3):
            cmps.append((hex(va(p)), hw & 0xFF))
    print(f"  va={hex(va(o))} BL={bls[:6]} CMP_imm={cmps[:8]}")

# STATUS path: does pin1==3 (DISABLED) skip SET#2 / force #5?
print("\n=== STATUS SET#5 / pin1 DISABLED skip (reconfirm) ===")
# known SET#5 @0x14fb5c6; STATUS prolog 0x14fb322
status = 0x14FB322
set5 = 0x14FB5C6
region = range(off_va(status) if False else (MAIN + (status - VA0)), MAIN + (0x14FB700 - VA0), 2)
# fix off helper
def off_v(v):
    return MAIN + (v - VA0)

print(f"  STATUS@{hex(status)} SET5@{hex(set5)}")
cmp3_near_set5 = []
for p in range(off_v(0x14FB322), off_v(0x14FB620), 2):
    hw = u16(p)
    if (hw & 0xFF00) == 0x2800 and (hw & 0xFF) == 3:
        cmp3_near_set5.append(hex(va(p)))
print(f"  CMP #3 in STATUS..SET5 window: {cmp3_near_set5}")

# Does SET_APP#5 path ever read pin1 byte?
# Search LDRB near SET5 for offsets that look like pin (prior: no)
print("\n=== Safe payload assessment ===")
print("Documented factory layout (RUNTIME):")
print("  id=0x020a len=72; SC@12=3; lock_mode@13; pwd_len@14; digits@15+; class@54=7; AID@55+")
print("  mode=1 enable (setIccLockEnabled true) — LIVE tried, err=2, pin1 stayed 2")
print("  mode=0 disable (setIccLockEnabled false) — needs PIN password in stock path")
print("  GET 0x0209 empty-pwd — LIVE ok, status=unlocked (byte13=0)")
print("If facility already unlocked, disable SET is not a pin1-moving lever.")
print("Empty-pwd SET disable is NOT documented as accepted; wrong-pwd may burn remain.")
print("pin1 DISABLED(3) does NOT skip Present==2 for SET#5 (ROADMAP/RUNTIME).")
print("DONE")
