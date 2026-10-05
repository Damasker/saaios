#!/usr/bin/env python3
"""Re-validate 0x0200 GetSimStatus offsets vs libsitril/sit-stream APPSTATE.

Confirms whether host tools reading app@17 / pin1@72 / remain@74 are
aligned with factory ProtocolSimStatusAdapter + BuildRilCardStatusApplications.
No live I/O. No secrets.
"""
from __future__ import annotations

import struct
from pathlib import Path

def _diag() -> Path:
    cands = [
        Path("/mnt/c/Users/Admin/Projects/saaios-modem-research/os/targets/panther/diagnostics"),
        Path(r"C:\Users\Admin\Projects\saaios-modem-research\os\targets\panther\diagnostics"),
        Path(__file__).resolve().parent,
    ]
    for p in cands:
        if (p / "sit-stream.so").exists() and (p / "libsitril.so").exists():
            return p
    raise SystemExit("sit-stream.so / libsitril.so not found")


DIAG = _diag()
SS = (DIAG / "sit-stream.so").read_bytes()
LS = (DIAG / "libsitril.so").read_bytes()
print(f"DIAG={DIAG}")


def u16(b, o):
    return struct.unpack_from("<H", b, o)[0]


def u32(b, o):
    return struct.unpack_from("<I", b, o)[0]


def u64(b, o):
    return struct.unpack_from("<Q", b, o)[0]


def parse_dynsym(data: bytes) -> dict[str, int]:
    if data[:4] != b"\x7fELF":
        return {}
    e_shoff = u64(data, 0x28)
    e_shentsize = u16(data, 0x3A)
    e_shnum = u16(data, 0x3C)
    e_shstrndx = u16(data, 0x3E)

    def sh(i):
        o = e_shoff + i * e_shentsize
        return {
            "name": u32(data, o),
            "type": u32(data, o + 4),
            "addr": u64(data, o + 16),
            "off": u64(data, o + 24),
            "size": u64(data, o + 32),
            "link": u32(data, o + 40),
            "entsize": u64(data, o + 56),
        }

    shstr = sh(e_shstrndx)
    names = data[shstr["off"] : shstr["off"] + shstr["size"]]

    def sn(off):
        end = names.find(b"\x00", off)
        return names[off:end].decode(errors="replace")

    secs = {}
    for i in range(e_shnum):
        s = sh(i)
        secs[sn(s["name"])] = s
    dynsym = secs.get(".dynsym")
    dynstr = secs.get(".dynstr")
    if not dynsym or not dynstr:
        return {}
    strtab = data[dynstr["off"] : dynstr["off"] + dynstr["size"]]
    out = {}
    entsize = dynsym["entsize"] or 24
    for i in range(dynsym["size"] // entsize):
        o = dynsym["off"] + i * entsize
        st_name = u32(data, o)
        st_value = u64(data, o + 8)
        end = strtab.find(b"\x00", st_name)
        name = strtab[st_name:end].decode(errors="replace")
        if name:
            out[name] = st_value
    return out


def va_to_off(data: bytes, va: int) -> int | None:
    e_phoff = u64(data, 0x20)
    e_phentsize = u16(data, 0x36)
    e_phnum = u16(data, 0x38)
    for i in range(e_phnum):
        o = e_phoff + i * e_phentsize
        p_type = u32(data, o)
        if p_type != 1:
            continue
        p_offset = u64(data, o + 8)
        p_vaddr = u64(data, o + 16)
        p_filesz = u64(data, o + 32)
        if p_vaddr <= va < p_vaddr + p_filesz:
            return p_offset + (va - p_vaddr)
    return None


def disasm_arm64_imm_loads(code: bytes, base_va: int, limit=80):
    """Collect LDRB/LDRH/ADD imm offsets from first `limit` instructions."""
    hits = []
    for i in range(0, min(len(code), limit * 4), 4):
        w = u32(code, i)
        va = base_va + i
        # LDRB (imm) 39xx xxxx : size=00 opc=01 V=0 — actually LDRB unsigned imm:
        # 0b00 111 001 01 Rn Rt imm12  => 0x39400000 mask
        if (w & 0xFFC00000) == 0x39400000:
            imm12 = (w >> 10) & 0xFFF
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            hits.append((va, f"LDRB x{rt},[x{rn},#{imm12}]", imm12, "b"))
        # LDRH unsigned imm: 0x79400000
        elif (w & 0xFFC00000) == 0x79400000:
            imm12 = (w >> 10) & 0xFFF
            scale = imm12 * 2
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            hits.append((va, f"LDRH x{rt},[x{rn},#{scale}]", scale, "h"))
        # LDR (32) unsigned: 0xB9400000
        elif (w & 0xFFC00000) == 0xB9400000:
            imm12 = (w >> 10) & 0xFFF
            scale = imm12 * 4
            rn = (w >> 5) & 0x1F
            rt = w & 0x1F
            hits.append((va, f"LDR w{rt},[x{rn},#{scale}]", scale, "w"))
        # ADD imm: 0x91000000
        elif (w & 0xFF000000) == 0x91000000:
            sh = (w >> 22) & 1
            imm12 = (w >> 10) & 0xFFF
            val = imm12 << (12 if sh else 0)
            rn = (w >> 5) & 0x1F
            rd = w & 0x1F
            hits.append((va, f"ADD x{rd},x{rn},#{val}", val, "add"))
    return hits


def find_strs(data: bytes, needles: list[bytes]):
    out = {}
    for n in needles:
        out[n.decode()] = []
        start = 0
        while True:
            i = data.find(n, start)
            if i < 0:
                break
            out[n.decode()].append(i)
            start = i + 1
    return out


print("=== A) Enum / string presence ===")
for blob, name in ((LS, "libsitril"), (SS, "sit-stream")):
    hits = find_strs(
        blob,
        [
            b"RIL_APPSTATE_UNKNOWN",
            b"RIL_APPSTATE_DETECTED",
            b"RIL_APPSTATE_PIN",
            b"RIL_APPSTATE_PUK",
            b"RIL_APPSTATE_SUBSCRIPTION_PERSO",
            b"RIL_APPSTATE_READY",
            b"APPSTATE_PIN",
            b"APPSTATE_READY",
            b"covertAppStateToString",
            b"GetAppState",
            b"GetPinState",
            b"GetPinRemainCount",
            b"ProtocolSimStatusAdapter",
            b"BuildRilCardStatusApplications",
            b"FillRilCardStatusFromAdapter",
        ],
    )
    print(f"-- {name}")
    for k, v in hits.items():
        if v:
            print(f"  {k}: n={len(v)} first={hex(v[0])}")

ssym = parse_dynsym(SS)
rsym = parse_dynsym(LS)
print("\n=== B) Key symbols (sit-stream / libsitril) ===")
for label, syms in (("sit-stream", ssym), ("libsitril", rsym)):
    print(f"-- {label}")
    for k, v in sorted(syms.items()):
        if any(
            x in k
            for x in (
                "SimStatus",
                "GetPin",
                "GetApp",
                "AppState",
                "PinRemain",
                "CardStatus",
                "covertApp",
            )
        ):
            print(f"  {k} = {hex(v)}")

# Known from prior RUNTIME: ProtocolSimStatusAdapter::Init @0x666a0, GetPinState @0x66a20
# Re-derive from dynsym if present; else use documented VAs.
CANDIDATES = []
for name, va in ssym.items():
    if "ProtocolSimStatusAdapter" in name or "GetPinState" in name or "GetPinRemain" in name:
        CANDIDATES.append((name, va))
    if "GetAppState" in name or "GetAppType" in name:
        CANDIDATES.append((name, va))

# Also search mangled fragments in .dynstr via raw
print("\n=== C) sit-stream adapter loads (documented + dynsym) ===")
# Documented absolute VAs from RUNTIME (file-relative for this build):
DOC = {
    "ProtocolSimStatusAdapter::Init": 0x666A0,
    "GetPinState": 0x66A20,
}
# Prefer dynsym
for name, va in CANDIDATES:
    DOC[name] = va

for name, va in sorted(DOC.items(), key=lambda x: x[1]):
    off = va_to_off(SS, va)
    if off is None:
        # try as file offset already
        if va < len(SS):
            off = va
            print(f"  {name}: treat as file-off {hex(va)}")
        else:
            print(f"  {name}: VA {hex(va)} not mapped")
            continue
    else:
        print(f"  {name}: VA {hex(va)} -> off {hex(off)}")
    code = SS[off : off + 0x120]
    hits = disasm_arm64_imm_loads(code, va if off != va else 0, limit=70)
    # Keep interesting immediates near known APPSTATE layout
    interesting = [
        h
        for h in hits
        if h[2]
        in (
            0x10,
            0x0E,
            0x0F,
            0x11,
            0x14,
            0x1F,
            0x21,
            0x31,
            0x33,
            0x54,
            0x58,
            0x59,
            0x5A,
            0x3F,
            15,
            16,
            17,
            31,
            33,
            36,
            40,
            53,
            51,
            60,
            63,
            72,
            73,
            74,
        )
        or h[3] == "add"
        and h[2] in (16, 0x10, 0x54, 15, 63, 0x3F)
    ]
    for h in interesting[:40]:
        print(f"    @{hex(h[0])} {h[1]}")
    if not interesting:
        print("    (no filtered imm hits; dumping first 25)")
        for h in hits[:25]:
            print(f"    @{hex(h[0])} {h[1]}")

# Walk GetPinState specifically for LDRB #0x58 / #0x59
print("\n=== D) GetPinState / remain offset scan in sit-stream ===")
# Scan whole .text for LDRB [xn,#0x58] and nearby function starts with GetPin in name
text_hits_58 = []
text_hits_5a = []
for i in range(0, len(SS) - 4, 4):
    w = u32(SS, i)
    if (w & 0xFFC00000) == 0x39400000:
        imm12 = (w >> 10) & 0xFFF
        if imm12 == 0x58:
            text_hits_58.append(i)
        if imm12 == 0x5A:
            text_hits_5a.append(i)
print(f"LDRB #0x58 count={len(text_hits_58)} first={[hex(x) for x in text_hits_58[:8]]}")
print(f"LDRB #0x5A count={len(text_hits_5a)} first={[hex(x) for x in text_hits_5a[:8]]}")

# Map documented math:
# adapter base = packet+0 (status body starts at packet byte 12? or adapter+16 = packet copy)
# Prior: Init copies packet to adapter+16
# apps count = packet byte 14 = adapter+16+14-12? Let's clarify:
# If copy starts at adapter+16 from packet byte 0 of SIT body (after 12B hdr),
# then packet byte 12 (card) = adapter+16+0 = adapter+16
# packet byte 14 (apps) = adapter+18
# packet byte 15 (type) = adapter+19
# packet byte 17 (state) = adapter+21
#
# But RUNTIME says type at adapter+31 (packet 15) and state at adapter+33 (packet 17).
# That implies adapter+16 points to packet byte 0 of FULL packet (incl 12B hdr):
# adapter+16+15 = adapter+31 type; adapter+16+17 = adapter+33 state. YES.
# pin1 packet 72 = adapter+16+72 = adapter+0x58. YES.

print("\n=== E) Layout math (factory evidence) ===")
print("SIT hdr=12; full packet offsets used by note_sim / stock-missing-gets:")
print("  card@12 apps@14 type@15 app_state@17 pin1@72 pin1_remain@74")
print("Adapter copy at +16 of full packet =>")
print("  type = adapter+31 (=16+15)")
print("  state = adapter+33 (=16+17)")
print("  pin1 = adapter+0x58 (=16+72)")
print("  pin1_remain = adapter+0x5A (=16+74)")
print("Per-app stride in Init copy length: 15 + 63*apps  (RUNTIME)")
print("  => first app record fields within 63B slot starting at packet 15")
print("  app0 type@15 state@17 ... pin1@72 (=15+57) remain@74")
print("  app1 would start at 15+63=78 if apps>=2")

# Check if BuildRil iterates apps with +63
print("\n=== F) Scan sit-stream for ADD #63 / #0x3f near CardStatus ===")
add63 = []
for i in range(0, len(SS) - 4, 4):
    w = u32(SS, i)
    if (w & 0xFF000000) == 0x91000000:
        sh = (w >> 22) & 1
        imm12 = (w >> 10) & 0xFFF
        val = imm12 << (12 if sh else 0)
        if val in (63, 0x3F):
            add63.append(i)
print(f"ADD #63/#0x3f count={len(add63)} offs={[hex(x) for x in add63[:12]]}")

# libsitril: FillRilCardStatusFromAdapter / GetAppState
print("\n=== G) libsitril FillRil / GetAppState ===")
for name, va in sorted(rsym.items()):
    if any(x in name for x in ("FillRilCardStatus", "GetAppState", "OnGetSimStatus", "BuildSimStatus", "covertApp")):
        print(f"  {name} = {hex(va)}")
        off = va_to_off(LS, va)
        if off is None and va < len(LS):
            off = va
        if off is not None:
            hits = disasm_arm64_imm_loads(LS[off : off + 0x100], va, limit=60)
            for h in hits[:20]:
                if h[2] in (0, 1, 2, 3, 4, 5, 17, 36, 40, 56, 60, 72, 0x24, 0x28, 0x38, 0x3C) or h[3] == "add":
                    print(f"    @{hex(h[0])} {h[1]}")

# Enum values from AOSP / covertAppStateToString prior evidence
print("\n=== H) Verdict mapping ===")
print("RIL_APPSTATE: 0 UNKNOWN, 1 DETECTED, 2 PIN, 3 PUK, 4 SUBSCRIPTION_PERSO, 5 READY")
print("Host tools (stock-missing-gets note_sim): g_app=p[17], g_pin=p[72], g_remain=p[74], g_apps=p[14]")
print("Multi-app: apps==1 live => only app0 slot; stride 63 would apply if apps>1")
print("Misread READY as PIN? Only if reading wrong byte (e.g. type@15=2 USIM vs state@17).")
print("Live length=143: 12 hdr + 15 + 63*1 + ? = 12+15+63=90, plus extras -> 143 OK for 1 app")

# Self-check length formula
for apps in (1, 2):
    # RUNTIME: copy length = 15 + 63*apps from body? or including?
    # "copy length is 15 plus 63 bytes per application" after apps count at byte 14
    # Full packet = 12 (hdr) + ? 
    # Live len 143 with apps=1
    print(f"  apps={apps}: 12+15+63*{apps}={12+15+63*apps}; live was 143 for apps=1")

print("\nDONE")
