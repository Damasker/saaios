#!/usr/bin/env python3
"""Find SimRefresh / adjacent signed SIM SITs we may not have sent."""
from __future__ import annotations

import struct
from pathlib import Path

SIT = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/sit-stream.so"
)
RIL = Path(
    "/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics/libsitril.so"
)


def dynsyms(data: bytes) -> dict[str, int]:
    if data[:4] != b"\x7fELF":
        return {}
    e_shoff = struct.unpack_from("<Q", data, 0x28)[0]
    e_shentsize = struct.unpack_from("<H", data, 0x3A)[0]
    e_shnum = struct.unpack_from("<H", data, 0x3C)[0]
    e_shstrndx = struct.unpack_from("<H", data, 0x3E)[0]

    def sh(i):
        o = e_shoff + i * e_shentsize
        return {
            "name": struct.unpack_from("<I", data, o)[0],
            "off": struct.unpack_from("<Q", data, o + 24)[0],
            "size": struct.unpack_from("<Q", data, o + 32)[0],
            "link": struct.unpack_from("<I", data, o + 40)[0],
            "entsize": struct.unpack_from("<Q", data, o + 56)[0],
        }

    shstr = sh(e_shstrndx)
    names = data[shstr["off"] : shstr["off"] + shstr["size"]]
    out = {}
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
                out[name] = st_value
    return out


def decode_id_len(data: bytes, addr: int, window: int = 0x100):
    """Heuristic MOVZ/MOVK W-reg for id + length stores (aarch64)."""
    end = min(len(data), addr + window)
    o = addr
    vals = []
    while o + 4 <= end:
        w = struct.unpack_from("<I", data, o)[0]
        # MOVZ Wd, #imm16
        if (w & 0xFF800000) == 0x52800000:
            imm = (w >> 5) & 0xFFFF
            rd = w & 0x1F
            vals.append(("MOVZ", rd, imm, o))
        # MOVK Wd, #imm16, LSL #16
        if (w & 0xFF800000) == 0x72A00000:
            imm = (w >> 5) & 0xFFFF
            rd = w & 0x1F
            vals.append(("MOVK16", rd, imm, o))
        o += 4
    # Prefer small ids in SIM/NET range
    ids = [v[2] for v in vals if v[0] == "MOVZ" and v[2] < 0x10000]
    return ids, vals[:20]


sit = SIT.read_bytes()
ril = RIL.read_bytes()
ss = dynsyms(sit)
rs = dynsyms(ril)

print("=== sit-stream symbols matching Refresh/Sim/File/ATR/Auth ===")
for n, v in sorted(ss.items(), key=lambda x: x[0]):
    low = n.lower()
    if any(
        k in low
        for k in (
            "refresh",
            "simfile",
            "getatr",
            "simstatus",
            "simget",
            "simset",
            "uicc",
            "cardpower",
            "facility",
            "openchannel",
            "transmit",
            "gba",
            "isim",
            "auth",
        )
    ):
        if "Build" in n or "ProtocolSim" in n:
            ids, _ = decode_id_len(sit, v)
            print(f"  {hex(v)} ids≈{ids[:8]}  {n[:110]}")

print("\n=== SimRefresh string contexts in sit-stream ===")
i = 0
shown = 0
while shown < 30:
    j = sit.find(b"Refresh", i)
    if j < 0:
        break
    a, b = max(0, j - 50), min(len(sit), j + 70)
    s = bytes(c if 32 <= c < 127 else 0x2E for c in sit[a:b]).decode()
    if "Sim" in s or "SIM" in s or "sim" in s or "RIL" in s:
        print(f"  {hex(j)} {s}")
        shown += 1
    i = j + 1

print("\n=== libsitril OnGetSimStatusDone / CheckAndAutoVerifyPin nearby strings ===")
for key in (
    b"OnGetSimStatusDone",
    b"CheckAndAutoVerifyPin",
    b"RIL_APPSTATE_READY",
    b"SimRefresh",
    b"SIM_REFRESH",
    b"RequestSimRefresh",
):
    print(f"  {key!r}: {hex(ril.find(key)) if ril.find(key)>=0 else None}")

# Inventory SIM opcodes from Build* we may not have live-sent after pin1=2
print("\n=== ProtocolSimBuilder* id heuristic dump ===")
for n, v in sorted(ss.items(), key=lambda x: x[0]):
    if "ProtocolSimBuilder" in n or "ProtocolSim" in n and "Build" in n:
        ids, _ = decode_id_len(sit, v)
        interesting = [i for i in ids if 0x200 <= i <= 0x260 or i in (0x4603,)]
        if interesting or "Refresh" in n:
            print(f"  {hex(v)} {interesting} {n[:100]}")
